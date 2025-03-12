use crate::ast::{
    AssignmentExpr, Ast, BinaryExpr, BlockStmt, BooleanExpr, ExpressionKind, IfStmt, LetStmt,
    NumberExpr, ParenthesizedExpr, StatementKind, UnaryExpr, VariableExpr,
};
use crate::source::span::TextSpan;

use super::{
    CallExpr, ExprId, Expression, FunctionDeclaration, ReturnStmt, StmtId, WhileStmt,
};

pub trait Visitor {
    fn get_ast(&self) -> &Ast;

    fn do_visit_statement(&mut self, statement: StmtId) {
        let statement = self.get_ast().query_stmt(statement).clone();
        match &statement.kind {
            StatementKind::Expression(expr) => {
                self.visit_expression(*expr);
            }
            StatementKind::Let(expr) => {
                self.visit_let_statement(expr);
            }
            StatementKind::If(stmt) => {
                self.visit_if_statement(stmt);
            }
            StatementKind::Block(stmt) => {
                self.visit_block_statement(stmt);
            }
            StatementKind::While(stmt) => {
                self.visit_while_statement(stmt);
            }
            StatementKind::FuncDecl(stmt) => {
                self.visit_func_decl_statement(stmt);
            }
            StatementKind::Return(stmt) => {
                self.visit_return_statement(stmt);
            }
        }
    }

    fn visit_while_statement(&mut self, while_statement: &WhileStmt) {
        self.visit_expression(while_statement.condition);
        self.visit_statement(while_statement.body);
    }

    fn visit_func_decl_statement(&mut self, func_decl_statement: &FunctionDeclaration) {
        self.visit_statement(func_decl_statement.body);
    }

    fn visit_return_statement(&mut self, return_statement: &ReturnStmt) {
        if let Some(expr) = &return_statement.return_value {
            self.visit_expression(*expr);
        }
    }

    fn visit_block_statement(&mut self, block_statement: &BlockStmt) {
        for statement in &block_statement.statements {
            self.visit_statement(*statement);
        }
    }

    fn visit_if_statement(&mut self, if_statement: &IfStmt) {
        self.visit_expression(if_statement.condition);
        self.visit_statement(if_statement.then_branch);
        if let Some(else_branch) = &if_statement.else_branch {
            self.visit_statement(else_branch.else_statement);
        }
    }

    fn visit_let_statement(&mut self, let_statement: &LetStmt);

    fn visit_statement(&mut self, statement: StmtId) {
        self.do_visit_statement(statement);
    }

    fn do_visit_expression(&mut self, expression: ExprId) {
        let expression = self.get_ast().query_expr(expression).clone();
        match &expression.kind {
            ExpressionKind::Number(number) => {
                self.visit_number_expression(number, &expression);
            }
            ExpressionKind::Binary(expr) => {
                self.visit_binary_expression(expr, &expression);
            }
            ExpressionKind::Parenthesized(expr) => {
                self.visit_parenthesized_expression(expr, &expression);
            }
            ExpressionKind::Error(span) => {
                self.visit_error(span);
            }
            ExpressionKind::Variable(expr) => {
                self.visit_variable_expression(expr, &expression);
            }
            ExpressionKind::Unary(expr) => {
                self.visit_unary_expression(expr, &expression);
            }
            ExpressionKind::Assignment(expr) => {
                self.visit_assignment_expression(expr, &expression);
            }
            ExpressionKind::Boolean(expr) => {
                self.visit_boolean_expression(expr, &expression);
            }
            ExpressionKind::Call(expr) => {
                self.visit_call_expression(expr, &expression);
            }
        }
    }

    fn visit_expression(&mut self, expression: ExprId) {
        self.do_visit_expression(expression);
    }

    fn visit_call_expression(&mut self, call_expression: &CallExpr, _expr: &Expression) {
        for argument in &call_expression.arguments {
            self.visit_expression(*argument);
        }
    }

    fn visit_assignment_expression(
        &mut self,
        assignment_expression: &AssignmentExpr,
        _expr: &Expression,
    ) {
        self.visit_expression(assignment_expression.expression);
    }

    fn visit_boolean_expression(&mut self, boolean: &BooleanExpr, _expr: &Expression);

    fn visit_variable_expression(
        &mut self,
        variable_expression: &VariableExpr,
        _expr: &Expression,
    );

    fn visit_number_expression(&mut self, number: &NumberExpr, _expr: &Expression);

    fn visit_error(&mut self, span: &TextSpan);

    fn visit_unary_expression(&mut self, unary_expression: &UnaryExpr, _expr: &Expression);

    fn visit_binary_expression(&mut self, binary_expression: &BinaryExpr, _expr: &Expression) {
        self.visit_expression(binary_expression.left);
        self.visit_expression(binary_expression.right);
    }

    fn visit_parenthesized_expression(
        &mut self,
        parenthesized_expression: &ParenthesizedExpr,
        _expr: &Expression,
    ) {
        self.visit_expression(parenthesized_expression.expression);
    }
}
