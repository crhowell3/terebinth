use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::evaluator::AstEvaluator;
use crate::ast::lexer::{Lexer, Token};
use crate::ast::parser::Parser;
use crate::ast::{
    AstBinaryOperatorKind, AstBooleanExpression, AstCallExpression, AstExpression,
    AstFuncDeclStatement, AstIfStatement, AstLetStatement, AstNumberExpression,
    AstParenthesizedExpression, AstReturnStatement, AstStmtId, AstUnaryExpression,
    AstUnaryOperatorKind, AstVariableExpression, AstWhileStatement,
};
use crate::ast::{AstBlockStatement, visitor::AstVisitor};
use crate::source::span::TextSpan;
use crate::typings::Type;
use crate::{ast, diagnostics, source};
use crate::{ast::Ast, diagnostics::DiagnosticsListCell};

#[derive(Debug, Clone)]
pub struct FunctionSymbol {
    pub parameters: Vec<VariableSymbol>,
    pub body: AstStmtId,
    pub return_type: Type,
}

#[derive(Debug, Clone)]
pub struct VariableSymbol {
    pub name: String,
    pub var_type: Type,
}

pub struct GlobalScope {
    variables: HashMap<String, VariableSymbol>,
    pub functions: HashMap<String, FunctionSymbol>,
}

impl GlobalScope {
    fn new() -> Self {
        GlobalScope {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    fn declare_variable(&mut self, identifier: &str, var_type: Type) {
        let variable = VariableSymbol {
            name: identifier.to_string(),
            var_type,
        };
        self.variables.insert(identifier.to_string(), variable);
    }

    fn lookup_variable(&self, identifier: &str) -> Option<&VariableSymbol> {
        self.variables.get(identifier)
    }

    fn declare_function(
        &mut self,
        identifier: &str,
        function_body: AstStmtId,
        parameters: Vec<VariableSymbol>,
        return_type: Type,
    ) -> Result<(), ()> {
        if self.functions.contains_key(identifier) {
            return Err(());
        }
        let function = FunctionSymbol {
            parameters,
            body: function_body,
            return_type,
        };

        self.functions.insert(identifier.to_string(), function);
        Ok(())
    }

    pub fn lookup_function(&self, identifier: &str) -> Option<&FunctionSymbol> {
        self.functions.get(identifier)
    }
}

struct LocalScope {
    variables: HashMap<String, VariableSymbol>,
    function: Option<FunctionSymbol>,
}

impl LocalScope {
    fn new(function: Option<FunctionSymbol>) -> Self {
        LocalScope {
            variables: HashMap::new(),
            function,
        }
    }

    fn declare_variable(&mut self, identifier: &str, var_type: Type) {
        let variable = VariableSymbol {
            name: identifier.to_string(),
            var_type,
        };
        self.variables.insert(identifier.to_string(), variable);
    }

    fn lookup_variable(&self, identifier: &str) -> Option<&VariableSymbol> {
        self.variables.get(identifier)
    }
}

struct Scopes {
    local_scopes: Vec<LocalScope>,
    global_scope: GlobalScope,
}

#[allow(dead_code)]
impl Scopes {
    fn new() -> Self {
        Scopes {
            local_scopes: Vec::new(),
            global_scope: GlobalScope::new(),
        }
    }

    fn from_global_scope(global_scope: GlobalScope) -> Self {
        Scopes {
            local_scopes: Vec::new(),
            global_scope,
        }
    }

    fn enter_scope(&mut self, function: Option<FunctionSymbol>) {
        self.local_scopes.push(LocalScope::new(function));
    }

    fn exit_scope(&mut self) {
        self.local_scopes.pop();
    }

    fn declare_variable(&mut self, identifier: &str, var_type: Type) {
        if self.is_inside_local_scope() {
            self.local_scopes
                .last_mut()
                .unwrap()
                .declare_variable(identifier, var_type);
        } else {
            self.global_scope.declare_variable(identifier, var_type);
        }
    }

    fn lookup_variable(&self, identifier: &str) -> Option<&VariableSymbol> {
        for scope in self.local_scopes.iter().rev() {
            if let Some(variable) = scope.lookup_variable(identifier) {
                return Some(variable);
            }
        }
        self.global_scope.lookup_variable(identifier)
    }

    fn lookup_function(&self, identifier: &str) -> Option<&FunctionSymbol> {
        self.global_scope.lookup_function(identifier)
    }

    fn is_inside_local_scope(&self) -> bool {
        !self.local_scopes.is_empty()
    }

    fn surrounding_function(&self) -> Option<&FunctionSymbol> {
        for scope in self.local_scopes.iter().rev() {
            if let Some(function) = &scope.function {
                return Some(function);
            }
        }
        None
    }
}

struct Resolver<'a> {
    scopes: Scopes,
    diagnostics: DiagnosticsListCell,
    ast: &'a mut Ast,
}

fn expect_type(diagnostics: &DiagnosticsListCell, expected: Type, actual: Type, span: &TextSpan) {
    if !actual.is_assignable_to(expected) {
        diagnostics
            .borrow_mut()
            .report_type_mismatch(span, expected, actual);
    }
}

impl<'a> Resolver<'a> {
    fn new(diagnostics: DiagnosticsListCell, scopes: Scopes, ast: &'a mut Ast) -> Self {
        Resolver {
            scopes,
            diagnostics,
            ast,
        }
    }

    pub fn resolve(&mut self) {
        let stmt_ids: Vec<AstStmtId> = self.ast.top_level_statements.clone();
        for stmt_id in stmt_ids {
            self.visit_statement(stmt_id);
        }
    }

    pub fn resolve_binary_expression(
        &self,
        left: &AstExpression,
        right: &AstExpression,
        operator: &AstBinaryOperatorKind,
    ) -> Type {
        let matrix: (Type, Type, Type) = match operator {
            AstBinaryOperatorKind::Plus
            | AstBinaryOperatorKind::Minus
            | AstBinaryOperatorKind::Multiply
            | AstBinaryOperatorKind::Divide
            | AstBinaryOperatorKind::Power
            | AstBinaryOperatorKind::BitwiseAnd
            | AstBinaryOperatorKind::BitwiseOr
            | AstBinaryOperatorKind::BitwiseXor
            | AstBinaryOperatorKind::LeftShift
            | AstBinaryOperatorKind::RightShift => (Type::Int, Type::Int, Type::Int),
            AstBinaryOperatorKind::Equals
            | AstBinaryOperatorKind::NotEquals
            | AstBinaryOperatorKind::LessThan
            | AstBinaryOperatorKind::LessThanOrEqual
            | AstBinaryOperatorKind::GreaterThan
            | AstBinaryOperatorKind::GreaterThanOrEqual => (Type::Int, Type::Int, Type::Bool),
        };

        self.expect_type(matrix.0, left.expr_type, &left.span(self.ast));
        self.expect_type(matrix.1, right.expr_type, &right.span(self.ast));

        matrix.2
    }

    fn expect_type(&self, expected: Type, actual: Type, span: &TextSpan) {
        expect_type(&self.diagnostics, expected, actual, span);
    }

    pub fn resolve_unary_expression(
        &self,
        operand: &AstExpression,
        operator: &AstUnaryOperatorKind,
    ) -> Type {
        let matrix: (Type, Type) = match operator {
            AstUnaryOperatorKind::Minus | AstUnaryOperatorKind::BitwiseNot => {
                (Type::Int, Type::Int)
            }
        };

        self.expect_type(matrix.0, operand.expr_type, &operand.span(self.ast));

        matrix.1
    }
}

fn resolve_type_from_string(diagnostics: &DiagnosticsListCell, type_name: &Token) -> Type {
    let lit_type = Type::from_str(&type_name.span.literal);
    let lit_type = match lit_type {
        None => {
            diagnostics.borrow_mut().report_undeclared_type(type_name);
            Type::Error
        }
        Some(lit_type) => lit_type,
    };
    lit_type
}

struct GlobalSymbolResolver<'a> {
    diagnostics: DiagnosticsListCell,
    global_scope: GlobalScope,
    ast: &'a Ast,
}

impl<'a> GlobalSymbolResolver<'a> {
    fn new(diagnostics: DiagnosticsListCell, ast: &'a Ast) -> Self {
        GlobalSymbolResolver {
            diagnostics,
            global_scope: GlobalScope::new(),
            ast,
        }
    }
}

impl AstVisitor for GlobalSymbolResolver<'_> {
    fn get_ast(&self) -> &Ast {
        self.ast
    }

    fn visit_func_decl_statement(&mut self, func_decl_statement: &AstFuncDeclStatement) {
        let parameters = func_decl_statement
            .parameters
            .iter()
            .map(|parameter| VariableSymbol {
                var_type: resolve_type_from_string(
                    &self.diagnostics,
                    &parameter.type_annotation.type_name,
                ),
                name: parameter.identifier.span.literal.clone(),
            })
            .collect();
        let literal_span = &func_decl_statement.identifier.span;
        let return_type = match &func_decl_statement.return_type {
            None => Type::Void,
            Some(return_type) => {
                resolve_type_from_string(&self.diagnostics, &return_type.type_name)
            }
        };
        match self.global_scope.declare_function(
            literal_span.literal.as_str(),
            func_decl_statement.body,
            parameters,
            return_type,
        ) {
            Ok(()) => {}
            Err(()) => {
                self.diagnostics
                    .borrow_mut()
                    .report_function_already_declared(&func_decl_statement.identifier);
            }
        }
    }

    fn visit_let_statement(&mut self, _let_statement: &ast::AstLetStatement) {}

    fn visit_variable_expression(
        &mut self,
        _variable_expression: &AstVariableExpression,
        _expr: &AstExpression,
    ) {
    }

    fn visit_number_expression(&mut self, _number: &AstNumberExpression, _expr: &AstExpression) {}

    fn visit_boolean_expression(&mut self, _boolean: &AstBooleanExpression, _expr: &AstExpression) {
    }

    fn visit_error(&mut self, _span: &TextSpan) {}

    fn visit_unary_expression(
        &mut self,
        _unary_expression: &AstUnaryExpression,
        _expr: &AstExpression,
    ) {
    }
}

impl AstVisitor for Resolver<'_> {
    fn get_ast(&self) -> &Ast {
        self.ast
    }

    fn visit_func_decl_statement(&mut self, func_decl_statement: &AstFuncDeclStatement) {
        let function_symbol = self
            .scopes
            .lookup_function(&func_decl_statement.identifier.span.literal)
            .unwrap()
            .clone();
        self.scopes.enter_scope(Some(function_symbol.clone()));
        for parameter in &function_symbol.parameters {
            self.scopes
                .declare_variable(&parameter.name, parameter.var_type);
        }
        self.visit_statement(func_decl_statement.body);
        self.scopes.exit_scope();
    }

    fn visit_return_statement(&mut self, return_statement: &AstReturnStatement) {
        let return_keyword = return_statement.return_keyword.clone();
        match self.scopes.surrounding_function().cloned() {
            None => {
                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                diagnostics_binding
                    .report_cannot_return_outside_function(&return_statement.return_keyword);
            }
            Some(function) => {
                if let Some(return_expression) = &return_statement.return_value {
                    self.visit_expression(*return_expression);
                    let return_expression = self.ast.query_expr(*return_expression);
                    self.expect_type(
                        function.return_type,
                        return_expression.expr_type,
                        &return_expression.span(self.ast),
                    );
                } else {
                    self.expect_type(Type::Void, function.return_type, &return_keyword.span);
                }
            }
        }
    }

    fn visit_while_statement(&mut self, while_statement: &AstWhileStatement) {
        self.visit_expression(while_statement.condition);
        let condition = self.ast.query_expr(while_statement.condition);
        self.expect_type(Type::Bool, condition.expr_type, &condition.span(self.ast));
        self.visit_statement(while_statement.body);
    }

    fn visit_block_statement(&mut self, block_statement: &AstBlockStatement) {
        self.scopes.enter_scope(None);
        for statement in &block_statement.statements {
            self.visit_statement(*statement);
        }
        self.scopes.exit_scope();
    }

    fn visit_if_statement(&mut self, if_statement: &AstIfStatement) {
        self.scopes.enter_scope(None);
        self.visit_expression(if_statement.condition);
        let condition_expression = self.ast.query_expr(if_statement.condition);
        self.expect_type(
            Type::Bool,
            condition_expression.expr_type,
            &condition_expression.span(self.ast),
        );
        self.visit_statement(if_statement.then_branch);
        self.scopes.exit_scope();
        if let Some(else_branch) = &if_statement.else_branch {
            self.scopes.enter_scope(None);
            self.visit_statement(else_branch.else_statement);
            self.scopes.exit_scope();
        }
    }

    fn visit_let_statement(&mut self, let_statement: &AstLetStatement) {
        let identifier = let_statement.identifier.span.literal.clone();
        self.visit_expression(let_statement.initializer);
        let initializer_expression = self.ast.query_expr(let_statement.initializer);
        let initializer_type = match &let_statement.type_annotation {
            Some(type_annotation) => {
                let ty = resolve_type_from_string(&self.diagnostics, &type_annotation.type_name);
                self.expect_type(
                    ty,
                    initializer_expression.expr_type,
                    &initializer_expression.span(self.ast),
                );
                ty
            }
            None => initializer_expression.expr_type,
        };
        self.scopes.declare_variable(&identifier, initializer_type);
    }

    fn visit_variable_expression(
        &mut self,
        variable_expression: &AstVariableExpression,
        expr: &AstExpression,
    ) {
        match self
            .scopes
            .lookup_variable(&variable_expression.identifier.span.literal)
        {
            None => {
                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                diagnostics_binding.report_undeclared_variable(&variable_expression.identifier);
            }
            Some(variable) => {
                self.ast.set_type(expr.id, variable.var_type);
            }
        }
    }

    fn visit_number_expression(&mut self, _number: &AstNumberExpression, expr: &AstExpression) {
        self.ast.set_type(expr.id, Type::Int);
    }

    fn visit_error(&mut self, _span: &TextSpan) {}

    fn visit_unary_expression(
        &mut self,
        unary_expression: &AstUnaryExpression,
        expr: &AstExpression,
    ) {
        self.visit_expression(unary_expression.operand);
        let operand = self.ast.query_expr(unary_expression.operand);
        let ty = self.resolve_unary_expression(operand, &unary_expression.operator.kind);
        self.ast.set_type(expr.id, ty);
    }

    fn visit_binary_expression(
        &mut self,
        binary_expression: &ast::AstBinaryExpression,
        expr: &AstExpression,
    ) {
        self.visit_expression(binary_expression.left);
        self.visit_expression(binary_expression.right);
        let left = self.ast.query_expr(binary_expression.left);
        let right = self.ast.query_expr(binary_expression.right);

        let ty = self.resolve_binary_expression(left, right, &binary_expression.operator.kind);
        self.ast.set_type(expr.id, ty);
    }

    fn visit_parenthesized_expression(
        &mut self,
        parenthesized_expression: &AstParenthesizedExpression,
        expr: &AstExpression,
    ) {
        self.visit_expression(parenthesized_expression.expression);

        let expression = self.ast.query_expr(parenthesized_expression.expression);

        self.ast.set_type(expr.id, expression.expr_type);
    }

    fn visit_boolean_expression(&mut self, _boolean: &AstBooleanExpression, _expr: &AstExpression) {
    }

    fn visit_call_expression(&mut self, call_expression: &AstCallExpression, expr: &AstExpression) {
        let function = self
            .scopes
            .lookup_function(&call_expression.identifier.span.literal)
            .cloned();
        let ty = match function {
            None => {
                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                diagnostics_binding.report_undeclared_function(&call_expression.identifier);
                Type::Void
            }
            Some(function) => {
                if function.parameters.len() != call_expression.arguments.len() {
                    let mut diagnostics_binding = self.diagnostics.borrow_mut();
                    diagnostics_binding.report_invalid_argument_count(
                        &call_expression.identifier,
                        function.parameters.len(),
                        call_expression.arguments.len(),
                    );
                }
                let return_type = function.return_type;
                for (argument, param) in call_expression
                    .arguments
                    .iter()
                    .zip(function.parameters.iter())
                {
                    self.visit_expression(*argument);
                    let argument_expression = self.ast.query_expr(*argument);
                    self.expect_type(
                        param.var_type,
                        argument_expression.expr_type,
                        &argument_expression.span(self.ast),
                    );
                }
                return_type
            }
        };
        self.ast.set_type(expr.id, ty);
    }
}

#[allow(dead_code)]
pub struct CompilationUnit {
    pub ast: Ast,
    pub diagnostics_list: DiagnosticsListCell,
    pub global_scope: GlobalScope,
}

impl CompilationUnit {
    pub fn compile(input: &str) -> Result<CompilationUnit, DiagnosticsListCell> {
        let text = source::SourceText::new(input.to_string());

        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();
        while let Some(token) = lexer.next_token() {
            tokens.push(token);
        }
        let diagnostics_list: DiagnosticsListCell =
            Rc::new(RefCell::new(diagnostics::DiagnosticsList::new()));
        let mut ast = Ast::new();
        let mut parser = Parser::new(&tokens, Rc::clone(&diagnostics_list), &mut ast);
        parser.parse();
        ast.visualize();

        Self::check_diagnostics(&text, &diagnostics_list)
            .map_err(|()| Rc::clone(&diagnostics_list))?;
        let mut global_symbol_resolver =
            GlobalSymbolResolver::new(Rc::clone(&diagnostics_list), &ast);
        ast.visit(&mut global_symbol_resolver);
        let global_scope = global_symbol_resolver.global_scope;
        let scopes = Scopes::from_global_scope(global_scope);
        let mut resolver = Resolver::new(Rc::clone(&diagnostics_list), scopes, &mut ast);
        resolver.resolve();
        Self::check_diagnostics(&text, &diagnostics_list)
            .map_err(|()| Rc::clone(&diagnostics_list))?;

        Ok(CompilationUnit {
            global_scope: resolver.scopes.global_scope,
            ast,
            diagnostics_list,
        })
    }

    pub fn run(&self) {
        let mut eval = AstEvaluator::new(&self.global_scope, &self.ast);
        let main_function = self.global_scope.lookup_function("main");
        if let Some(function) = main_function {
            eval.visit_statement(function.body);
        } else {
            self.ast.visit(&mut eval);
        }
        println!("Result: {:?}", eval.last_value);
    }

    fn check_diagnostics(
        text: &source::SourceText,
        diagnostics_list: &DiagnosticsListCell,
    ) -> Result<(), ()> {
        let diagnostics_binding = diagnostics_list.borrow();
        if !diagnostics_binding.diagnostics.is_empty() {
            let diagnostics_printer = diagnostics::printer::DiagnosticsPrinter::new(
                text,
                &diagnostics_binding.diagnostics,
            );
            diagnostics_printer.print();
            return Err(());
        }
        Ok(())
    }
}
