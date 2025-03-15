use std::cell::RefCell;
use std::rc::Rc;

use terebinth::{Idx, IdxVec, idx};

use crate::ast::evaluator::AstEvaluator;
use crate::ast::lexer::{Lexer, Token};
use crate::ast::parser::Parser;
use crate::ast::{
    BinaryOperatorKind, BooleanExpr, CallExpr, Expr, FunctionDeclaration, IfExpr, LetStmt,
    NumberExpr, ParenthesizedExpr, ReturnStmt, StmtId, UnaryExpr, UnaryOperatorKind, VariableExpr,
    WhileStmt,
};
use crate::ast::{BlockExpr, visitor::Visitor};
use crate::source::span::TextSpan;
use crate::typings::Type;
use crate::{ast, diagnostics, source};
use crate::{ast::Ast, diagnostics::DiagnosticsListCell};

idx!(FunctionIndex);
idx!(VariableIndex);

#[derive(Debug, Clone)]
pub struct FunctionSymbol {
    pub parameters: Vec<VariableIndex>,
    pub body: StmtId,
    pub return_type: Type,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct VariableSymbol {
    pub name: String,
    pub var_type: Type,
}

pub struct GlobalScope {
    pub variables: IdxVec<VariableIndex, VariableSymbol>,
    pub functions: IdxVec<FunctionIndex, FunctionSymbol>,
    pub global_variables: Vec<VariableIndex>,
}

impl GlobalScope {
    fn new() -> Self {
        GlobalScope {
            variables: IdxVec::new(),
            functions: IdxVec::new(),
            global_variables: Vec::new(),
        }
    }

    fn declare_variable(
        &mut self,
        identifier: &str,
        var_type: Type,
        is_global: bool,
    ) -> VariableIndex {
        let variable = VariableSymbol {
            name: identifier.to_string(),
            var_type,
        };
        let variable_idx = self.variables.push(variable);
        if is_global {
            self.global_variables.push(variable_idx);
        }
        variable_idx
    }

    fn lookup_global_variable(&self, identifier: &str) -> Option<&VariableIndex> {
        self.global_variables
            .iter()
            .map(|variable_idx| (variable_idx, self.variables.get(*variable_idx)))
            .find(|(_, variable)| variable.name == identifier)
            .map(|(variable_idx, _)| variable_idx)
    }

    fn declare_function(
        &mut self,
        identifier: &str,
        function_body: StmtId,
        parameters: Vec<VariableIndex>,
        return_type: Type,
    ) -> Result<(), ()> {
        if self.lookup_function(identifier).is_some() {
            return Err(());
        }
        let function = FunctionSymbol {
            parameters,
            body: function_body,
            return_type,
            name: identifier.to_string(),
        };

        self.functions.push(function);
        Ok(())
    }

    pub fn lookup_function(&self, identifier: &str) -> Option<FunctionIndex> {
        self.functions
            .indexed_iter()
            .find(|(_, function)| function.name == identifier)
            .map(|(idx, _)| idx)
    }
}

struct LocalScope {
    locals: Vec<VariableIndex>,
}

impl LocalScope {
    fn new() -> Self {
        LocalScope { locals: Vec::new() }
    }

    fn add_local(&mut self, local: VariableIndex) {
        self.locals.push(local);
    }
}

struct Scopes {
    local_scopes_vec: Vec<LocalScope>,
    global_scope: GlobalScope,
    surrounding_function: Option<FunctionIndex>,
}

#[allow(dead_code)]
impl Scopes {
    fn new() -> Self {
        Scopes {
            local_scopes_vec: Vec::new(),
            global_scope: GlobalScope::new(),
            surrounding_function: None,
        }
    }

    fn from_global_scope(global_scope: GlobalScope) -> Self {
        Scopes {
            local_scopes_vec: Vec::new(),
            global_scope,
            surrounding_function: None,
        }
    }

    fn enter_function_scope(&mut self, function_idx: FunctionIndex) {
        self.surrounding_function = Some(function_idx);
        self.enter_scope();
    }

    fn enter_scope(&mut self) {
        self.local_scopes_vec.push(LocalScope::new());
    }

    fn exit_function_scope(&mut self) {
        self.surrounding_function = None;
        self.exit_scope();
    }

    fn exit_scope(&mut self) {
        self.local_scopes_vec.pop();
    }

    fn declare_variable(&mut self, identifier: &str, var_type: Type) {
        let is_inside_local_scope = self.is_inside_local_scope();
        let idx = self
            .global_scope
            .declare_variable(identifier, var_type, !is_inside_local_scope);
        if is_inside_local_scope {
            self.current_local_scope_mut().add_local(idx);
        }
    }

    fn lookup_variable(&self, identifier: &str) -> Option<VariableIndex> {
        for scope in self.local_scopes_vec.iter().rev() {
            if let Some((idx, _variable)) = scope
                .locals
                .iter()
                .map(|idx| (*idx, self.global_scope.variables.get(*idx)))
                .find(|(_idx, variable)| variable.name == identifier)
            {
                return Some(idx);
            }
        }
        self.global_scope
            .lookup_global_variable(identifier)
            .copied()
    }

    fn lookup_function(&self, identifier: &str) -> Option<FunctionIndex> {
        self.global_scope.lookup_function(identifier)
    }

    fn is_inside_local_scope(&self) -> bool {
        !self.local_scopes_vec.is_empty()
    }

    fn surrounding_function(&self) -> Option<&FunctionSymbol> {
        self.surrounding_function
            .map(|idx| self.global_scope.functions.get(idx))
    }

    fn current_local_scope_mut(&mut self) -> &mut LocalScope {
        self.local_scopes_vec.last_mut().unwrap()
    }
}

struct Resolver {
    scopes: Scopes,
    diagnostics: DiagnosticsListCell,
}

fn expect_type(
    diagnostics: &DiagnosticsListCell,
    expected: Type,
    actual: Type,
    span: &TextSpan,
) -> Type {
    if !actual.is_assignable_to(expected) {
        diagnostics
            .borrow_mut()
            .report_type_mismatch(span, expected, actual);
    }
    expected
}

impl Resolver {
    fn new(diagnostics: DiagnosticsListCell, scopes: Scopes) -> Self {
        Resolver {
            scopes,
            diagnostics,
        }
    }

    pub fn resolve(&mut self, ast: &mut Ast) {
        for id in ast.items.cloned_indices() {
            self.visit_item(ast, id);
        }
    }

    pub fn resolve_binary_expression(
        &self,
        left: &Expr,
        right: &Expr,
        operator: &BinaryOperatorKind,
    ) -> Type {
        let matrix: (Type, Type, Type) = match operator {
            BinaryOperatorKind::Plus
            | BinaryOperatorKind::Minus
            | BinaryOperatorKind::Multiply
            | BinaryOperatorKind::Divide
            | BinaryOperatorKind::Power
            | BinaryOperatorKind::BitwiseAnd
            | BinaryOperatorKind::BitwiseOr
            | BinaryOperatorKind::BitwiseXor
            | BinaryOperatorKind::LeftShift
            | BinaryOperatorKind::RightShift => (Type::Int, Type::Int, Type::Int),
            BinaryOperatorKind::Equals
            | BinaryOperatorKind::NotEquals
            | BinaryOperatorKind::LessThan
            | BinaryOperatorKind::LessThanOrEqual
            | BinaryOperatorKind::GreaterThan
            | BinaryOperatorKind::GreaterThanOrEqual => (Type::Int, Type::Int, Type::Bool),
        };

        self.expect_type(matrix.0, left.expr_type, &left.span(self.ast));
        self.expect_type(matrix.1, right.expr_type, &right.span(self.ast));

        matrix.2
    }

    fn expect_type(&self, expected: Type, actual: Type, span: &TextSpan) {
        expect_type(&self.diagnostics, expected, actual, span);
    }

    pub fn resolve_unary_expression(&self, operand: &Expr, operator: &UnaryOperatorKind) -> Type {
        let matrix: (Type, Type) = match operator {
            UnaryOperatorKind::Minus | UnaryOperatorKind::BitwiseNot => (Type::Int, Type::Int),
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

impl Visitor for GlobalSymbolResolver<'_> {
    fn get_ast(&self) -> &Ast {
        self.ast
    }

    fn visit_func_decl_statement(&mut self, func_decl_statement: &FunctionDeclaration) {
        let parameters = func_decl_statement
            .parameters
            .iter()
            .map(|parameter| {
                self.global_scope.declare_variable(
                    &parameter.identifier.span.literal,
                    resolve_type_from_string(
                        &self.diagnostics,
                        &parameter.type_annotation.type_name,
                    ),
                    false,
                )
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

    fn visit_let_statement(&mut self, _let_statement: &ast::LetStmt) {}

    fn visit_variable_expression(&mut self, _variable_expression: &VariableExpr, _expr: &Expr) {}

    fn visit_number_expression(&mut self, _number: &NumberExpr, _expr: &Expr) {}

    fn visit_boolean_expression(&mut self, _boolean: &BooleanExpr, _expr: &Expr) {}

    fn visit_error(&mut self, _span: &TextSpan) {}

    fn visit_unary_expression(&mut self, _unary_expression: &UnaryExpr, _expr: &Expr) {}
}

impl Visitor for Resolver<'_> {
    fn get_ast(&self) -> &Ast {
        self.ast
    }

    fn visit_func_decl_statement(&mut self, func_decl_statement: &FunctionDeclaration) {
        let function_id = self
            .scopes
            .lookup_function(&func_decl_statement.identifier.span.literal)
            .unwrap();
        self.scopes.enter_function_scope(function_id);
        let function = self.scopes.global_scope.functions.get(&function_id);
        for parameter in function.parameters.clone() {
            self.scopes.current_local_scope_mut().locals.push(parameter);
        }
        self.visit_stmt(func_decl_statement.body);
        self.scopes.exit_function_scope();
    }

    fn visit_return_statement(&mut self, return_statement: &ReturnStmt) {
        let return_keyword = return_statement.return_keyword.clone();
        match self.scopes.surrounding_function().cloned() {
            None => {
                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                diagnostics_binding
                    .report_cannot_return_outside_function(&return_statement.return_keyword);
            }
            Some(function) => {
                if let Some(return_expression) = &return_statement.return_value {
                    self.visit_expr(*return_expression);
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

    fn visit_while_statement(&mut self, while_statement: &WhileStmt) {
        self.visit_expr(while_statement.condition);
        let condition = self.ast.query_expr(while_statement.condition);
        self.expect_type(Type::Bool, condition.expr_type, &condition.span(self.ast));
        self.visit_stmt(while_statement.body);
    }

    fn visit_block_expr(&mut self, block_statement: &BlockExpr) {
        self.scopes.enter_scope();
        for statement in &block_statement.stmts {
            self.visit_stmt(*statement);
        }
        self.scopes.exit_scope();
    }

    fn visit_if_expr(&mut self, if_statement: &IfExpr) {
        self.scopes.enter_scope();
        self.visit_expr(if_statement.condition);
        let condition_expression = self.ast.query_expr(if_statement.condition);
        self.expect_type(
            Type::Bool,
            condition_expression.expr_type,
            &condition_expression.span(self.ast),
        );
        self.visit_stmt(if_statement.then_branch);
        self.scopes.exit_scope();
        if let Some(else_branch) = &if_statement.else_branch {
            self.scopes.enter_scope();
            self.visit_stmt(else_branch.else_statement);
            self.scopes.exit_scope();
        }
    }

    fn visit_let_statement(&mut self, let_statement: &LetStmt) {
        let identifier = let_statement.identifier.span.literal.clone();
        self.visit_expr(let_statement.initializer);
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

    fn visit_variable_expression(&mut self, variable_expression: &VariableExpr, expr: &Expr) {
        match self
            .scopes
            .lookup_variable(&variable_expression.identifier.span.literal)
        {
            None => {
                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                diagnostics_binding.report_undeclared_variable(&variable_expression.identifier);
            }
            Some(variable) => {
                let variable = self.scopes.global_scope.variables.get(variable);
                self.ast.set_type(expr.id, variable.var_type);
            }
        }
    }

    fn visit_number_expression(&mut self, _number: &NumberExpr, expr: &Expr) {
        self.ast.set_type(expr.id, Type::Int);
    }

    fn visit_error(&mut self, _span: &TextSpan) {}

    fn visit_unary_expression(&mut self, unary_expression: &UnaryExpr, expr: &Expr) {
        self.visit_expr(unary_expression.operand);
        let operand = self.ast.query_expr(unary_expression.operand);
        let ty = self.resolve_unary_expression(operand, &unary_expression.operator.kind);
        self.ast.set_type(expr.id, ty);
    }

    fn visit_binary_expression(&mut self, binary_expression: &ast::BinaryExpr, expr: &Expr) {
        self.visit_expr(binary_expression.left);
        self.visit_expr(binary_expression.right);
        let left = self.ast.query_expr(binary_expression.left);
        let right = self.ast.query_expr(binary_expression.right);

        let ty = self.resolve_binary_expression(left, right, &binary_expression.operator.kind);
        self.ast.set_type(expr.id, ty);
    }

    fn visit_parenthesized_expression(
        &mut self,
        parenthesized_expression: &ParenthesizedExpr,
        expr: &Expr,
    ) {
        self.visit_expr(parenthesized_expression.expression);

        let expression = self.ast.query_expr(parenthesized_expression.expression);

        self.ast.set_type(expr.id, expression.expr_type);
    }

    fn visit_boolean_expression(&mut self, _boolean: &BooleanExpr, _expr: &Expr) {}

    fn visit_call_expression(&mut self, call_expression: &CallExpr, expr: &Expr) {
        let function = self
            .scopes
            .lookup_function(&call_expression.identifier.span.literal);
        let ty = match function {
            None => {
                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                diagnostics_binding.report_undeclared_function(&call_expression.identifier);
                Type::Void
            }
            Some(function) => {
                let function = self.scopes.global_scope.functions.get(&function);
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
                    .zip(function.parameters.clone().iter())
                {
                    self.visit_expr(*argument);
                    let argument_expression = self.ast.query_expr(*argument);
                    let param = self.scopes.global_scope.variables.get(param);
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

    fn visit_assignment_expression(
        &mut self,
        assignment_expression: &ast::AssignmentExpr,
        expr: &Expr,
    ) {
        self.visit_expr(assignment_expression.expression);
        let value_expression = self.ast.query_expr(assignment_expression.expression);
        let identifier = assignment_expression.identifier.span.literal.clone();
        let ty = match self.scopes.lookup_variable(&identifier) {
            None => {
                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                diagnostics_binding.report_undeclared_variable(&assignment_expression.identifier);
                Type::Void
            }
            Some(variable) => {
                let variable = self.scopes.global_scope.variables.get(variable);
                self.expect_type(
                    variable.var_type,
                    value_expression.expr_type,
                    &value_expression.span(self.ast),
                );
                variable.var_type
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
            let function = self.global_scope.functions.get(&function);
            eval.visit_stmt(function.body);
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
