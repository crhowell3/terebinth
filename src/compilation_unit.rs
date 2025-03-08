use std::cell::RefCell;
use std::collections::HashMap;
use std::env::var;
use std::rc::Rc;

use termion::color::LightBlack;

use crate::ast::evaluator::AstEvaluator;
use crate::ast::lexer::{Lexer, TextSpan};
use crate::ast::parser::Parser;
use crate::ast::{
    AstBinaryOperatorKind, AstExpression, AstFuncDeclStatement, AstIfStatement, AstStatement,
    AstStmtId, AstUnaryOperatorKind,
};
use crate::ast::{AstBlockStatement, visitor::AstVisitor};
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
        function_body: &AstStmtId,
        parameters: Vec<VariableSymbol>,
        return_type: Type,
    ) -> Result<(), ()> {
        if self.functions.contains_key(identifier) {
            return Err(());
        }
        let function = FunctionSymbol {
            parameters,
            body: function_body.clone(),
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

fn expect_type(diagnostics: &DiagnosticsListCell, expected: Type, actual: &Type, span: &TextSpan) {
    if !actual.is_assignable_to(&expected) {
        diagnostics
            .borrow_mut()
            .report_type_mismatch(span, &expected, actual);
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
        let stmt_ids: Vec<AstStmtId> = self
            .ast
            .top_level_statements
            .iter()
            .map(|stmt| stmt.clone())
            .collect();
        for stmt_id in stmt_ids {
            self.visit_statement(&stmt_id);
        }
    }

    pub fn resolve_binary_expression(
        &self,
        left: &AstExpression,
        right: &AstExpression,
        operator: &AstBinaryOperatorKind,
    ) -> Type {
        let matrix: (Type, Type, Type) = match operator {
            AstBinaryOperatorKind::Plus => (Type::Int, Type::Int, Type::Int),
            AstBinaryOperatorKind::Minus => (Type::Int, Type::Int, Type::Int),
            AstBinaryOperatorKind::Multiply => (Type::Int, Type::Int, Type::Int),
            AstBinaryOperatorKind::Divide => (Type::Int, Type::Int, Type::Int),
            AstBinaryOperatorKind::Power => (Type::Int, Type::Int, Type::Int),
            AstBinaryOperatorKind::BitwiseAnd => (Type::Int, Type::Int, Type::Int),
            AstBinaryOperatorKind::BitwiseOr => (Type::Int, Type::Int, Type::Int),
            AstBinaryOperatorKind::BitwiseXor => (Type::Int, Type::Int, Type::Int),
            AstBinaryOperatorKind::Equals => (Type::Int, Type::Int, Type::Bool),
            AstBinaryOperatorKind::NotEquals => (Type::Int, Type::Int, Type::Bool),
            AstBinaryOperatorKind::LessThan => (Type::Int, Type::Int, Type::Bool),
            AstBinaryOperatorKind::LessThanOrEqual => (Type::Int, Type::Int, Type::Bool),
            AstBinaryOperatorKind::GreaterThan => (Type::Int, Type::Int, Type::Bool),
            AstBinaryOperatorKind::GreaterThanOrEqual => (Type::Int, Type::Int, Type::Bool),
        };

        self.expect_type(matrix.0, &left.expr_type, &left.span(&self.ast));
        self.expect_type(matrix.1, &right.expr_type, &right.span(&self.ast));

        matrix.2
    }

    fn expect_type(&self, expected: Type, actual: &Type, span: &TextSpan) {
        expect_type(&self.diagnostics, expected, actual, span);
    }

    pub fn resolve_unary_expression(
        &self,
        operand: &AstExpression,
        operator: &AstUnaryOperatorKind,
    ) -> Type {
        let matrix: (Type, Type) = match operator {
            AstUnaryOperatorKind::Minus => (Type::Int, Type::Int),
            AstUnaryOperatorKind::BitwiseNot => (Type::Int, Type::Int),
        };

        self.expect_type(matrix.0, &operand.expr_type, &operand.span(&self.ast));

        matrix.1
    }
}

fn resolve_type_from_string(diagnostics: &DiagnosticsListCell, type_name: &Token) -> Type {
    let lit_type = Type::from_str(&type_name.span.literal);
    let lit_type = match lit_type {
        None => {
            diagnostics.borrow_mut().report_undeclared_type(&type_name);
            Type::Error
        }
        Some(lit_type) => lit_type,
    };
}

struct GlobalSymbolResolver {
    diagnostics: DiagnosticsListCell,
    global_scope: GlobalScope,
}

impl GlobalSymbolResolver {
    fn new(diagnostics: DiagnosticsListCell) -> Self {
        GlobalSymbolResolver {
            diagnostics,
            global_scope: GlobalScope::new(),
        }
    }
}

impl AstVisitor<'_> for GlobalSymbolResolver {
    fn visit_func_decl_statement(&mut self, func_decl_statement: &AstFuncDeclStatement) {
        let parameters = func_decl_statement
            .parameters
            .iter()
            .map(|parameter| parameter.identifier.span.literal.clone())
            .collect();
        let literal_span = &func_decl_statement.identifier.span;
        match self.global_scope.declare_function(
            &literal_span.literal.as_str(),
            &func_decl_statement.body,
            parameters,
        ) {
            Ok(_) => {}
            Err(_) => {
                self.diagnostics
                    .borrow_mut()
                    .report_function_already_declared(&func_decl_statement.identifier);
            }
        }
    }

    fn visit_let_statement(&mut self, let_statement: &ast::AstLetStatement) {}

    fn visit_variable_expression(&mut self, variable_expression: &ast::AstVariableExpression) {}

    fn visit_number_expression(&mut self, number: &ast::AstNumberExpression) {}

    fn visit_boolean_expression(&mut self, boolean: &ast::AstBooleanExpression) {}

    fn visit_error(&mut self, span: &ast::lexer::TextSpan) {}

    fn visit_unary_expression(&mut self, unary_expression: &ast::AstUnaryExpression) {}
}

impl AstVisitor<'_> for Resolver {
    fn visit_block_statement(&mut self, block_statement: &AstBlockStatement) {
        self.scopes.enter_scope();
        for statement in &block_statement.statements {
            self.visit_statement(statement);
        }
        self.scopes.exit_scope();
    }

    fn visit_let_statement(&mut self, let_statement: &ast::AstLetStatement) {
        let identifier = let_statement.identifier.span.literal.clone();
        self.visit_expression(&let_statement.initializer);
        self.scopes.declare_variable(&identifier);
    }

    fn visit_variable_expression(&mut self, variable_expression: &ast::AstVariableExpression) {
        if !self
            .scopes
            .lookup_variable(&variable_expression.identifier.span.literal)
        {
            let mut diagnostics_binding = self.diagnostics.borrow_mut();
            diagnostics_binding.report_undeclared_variable(&variable_expression.identifier);
        }
    }

    fn visit_number_expression(&mut self, _number: &ast::AstNumberExpression) {}

    fn visit_error(&mut self, _span: &ast::lexer::TextSpan) {}

    fn visit_unary_expression(&mut self, unary_expression: &ast::AstUnaryExpression) {
        self.visit_expression(&unary_expression.operand);
    }

    fn visit_boolean_expression(&mut self, boolean: &ast::AstBooleanExpression) {}

    fn visit_if_statement(&mut self, if_statement: &AstIfStatement) {
        self.scopes.enter_scope();
        self.visit_expression(&if_statement.condition);
        self.visit_statement(&if_statement.then_branch);
        self.scopes.exit_scope();
        if let Some(else_branch) = &if_statement.else_branch {
            self.scopes.enter_scope();
            self.visit_statement(&else_branch.else_statement);
            self.scopes.exit_scope();
        }
    }

    fn visit_func_decl_statement(&mut self, func_decl_statement: &AstFuncDeclStatement) {
        self.scopes.enter_scope();
        for parameter in &func_decl_statement.parameters {
            self.scopes
                .declare_variable(&parameter.identifier.span.literal);
        }
        self.visit_statement(&func_decl_statement.body);
        self.scopes.exit_scope();
    }

    fn visit_call_expression(&mut self, call_expression: &ast::AstCallExpression) {
        let function = self
            .scopes
            .lookup_function(&call_expression.identifier.span.literal);
        match function {
            None => {
                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                diagnostics_binding.report_undeclared_function(&call_expression.identifier);
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
            }
        }
        for argument in &call_expression.arguments {
            self.visit_expression(argument);
        }
    }
}

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
        let mut parser = Parser::new(tokens, Rc::clone(&diagnostics_list));
        while let Some(statement) = parser.next_statement() {
            ast.add_statement(statement);
        }
        ast.visualize();

        Self::check_diagnostics(&text, &diagnostics_list)
            .map_err(|_| Rc::clone(&diagnostics_list))?;
        let mut global_symbol_resolver = GlobalSymbolResolver::new(Rc::clone(&diagnostics_list));
        ast.visit(&mut global_symbol_resolver);
        let global_scope = global_symbol_resolver.global_scope;
        let scopes = Scopes::from_global_scope(global_scope);
        let mut resolver = Resolver::new(Rc::clone(&diagnostics_list), scopes);
        ast.visit(&mut resolver);
        Self::check_diagnostics(&text, &diagnostics_list)
            .map_err(|_| Rc::clone(&diagnostics_list))?;

        Ok(CompilationUnit {
            ast,
            diagnostics_list,
            global_scope: resolver.scopes.global_scope,
        })
    }

    pub fn maybe_run(&self) {
        if self.diagnostics_list.borrow().diagnostics.len() > 0 {
            return;
        }
        self.run();
    }

    pub fn run(&self) {
        let mut eval = AstEvaluator::new(&self.global_scope);
        let main_function = self.global_scope.lookup_function("main");
        if let Some(function) = main_function {
            eval.visit_statement(&function.body);
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
