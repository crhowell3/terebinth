use crate::ast::lexer::TextSpan;
use crate::ast::{
    Ast, AstAssignmentExpression, AstBinaryExpression, AstBlockStatement, AstBooleanExpression,
    AstExpressionKind, AstIfStatement, AstLetStatement, AstNumberExpression,
    AstParenthesizedExpression, AstStatementKind, AstUnaryExpression, AstVariableExpression,
};

use super::{
    AstCallExpression, AstExprId, AstFuncDeclStatement, AstReturnStatement, AstStmtId,
    AstWhileStatement,
};

pub trait AstVisitor {
    fn get_ast(&self) -> &Ast;

    fn do_visit_statement(&mut self, statement: &AstStmtId) {
        let statement = self.get_ast().query_stmt(statement).clone();
        match &statement.kind {
            AstStatementKind::Expression(expr) => {
                self.visit_expression(expr);
            }
            AstStatementKind::Let(expr) => {
                self.visit_let_statement(expr);
            }
            AstStatementKind::If(stmt) => {
                self.visit_if_statement(stmt);
            }
            AstStatementKind::Block(stmt) => {
                self.visit_block_statement(stmt);
            }
            AstStatementKind::While(stmt) => {
                self.visit_while_statement(stmt);
            }
            AstStatementKind::FuncDecl(stmt) => {
                self.visit_func_decl_statement(stmt);
            }
            AstStatementKind::Return(stmt) => {
                self.visit_return_statement(stmt);
            }
        }
    }

    fn visit_while_statement(&mut self, while_statement: &AstWhileStatement) {
        self.visit_expression(&while_statement.condition);
        self.visit_statement(&while_statement.body);
    }

    fn visit_func_decl_statement(&mut self, func_decl_statement: &AstFuncDeclStatement) {
        self.visit_statement(&func_decl_statement.body);
    }

    fn visit_return_statement(&mut self, return_statement: &AstReturnStatement) {
        if let Some(expr) = &return_statement.return_value {
            self.visit_expression(expr);
        }
    }

    fn visit_block_statement(&mut self, block_statement: &AstBlockStatement) {
        for statement in &block_statement.statements {
            self.visit_statement(statement);
        }
    }

    fn visit_if_statement(&mut self, if_statement: &AstIfStatement) {
        self.visit_expression(&if_statement.condition);
        self.visit_statement(&if_statement.then_branch);
        if let Some(else_branch) = &if_statement.else_branch {
            self.visit_statement(&else_branch.else_statement);
        }
    }

    fn visit_let_statement(&mut self, let_statement: &AstLetStatement);

    fn visit_statement(&mut self, statement: &AstStmtId) {
        self.do_visit_statement(statement);
    }

    fn do_visit_expression(&mut self, expression: &AstExprId) {
        let expression = self.get_ast().query_expr(expression).clone();
        match &expression.kind {
            AstExpressionKind::Number(number) => {
                self.visit_number_expression(number);
            }
            AstExpressionKind::Binary(expr) => {
                self.visit_binary_expression(expr);
            }
            AstExpressionKind::Parenthesized(expr) => {
                self.visit_parenthesized_expression(expr);
            }
            AstExpressionKind::Error(span) => {
                self.visit_error(span);
            }
            AstExpressionKind::Variable(expr) => {
                self.visit_variable_expression(expr);
            }
            AstExpressionKind::Unary(expr) => {
                self.visit_unary_expression(expr);
            }
            AstExpressionKind::Assignment(expr) => {
                self.visit_assignment_expression(expr);
            }
            AstExpressionKind::Boolean(expr) => {
                self.visit_boolean_expression(expr);
            }
            AstExpressionKind::Call(expr) => {
                self.visit_call_expression(expr);
            }
        }
    }

    fn visit_expression(&mut self, expression: &AstExprId) {
        self.do_visit_expression(expression);
    }

    fn visit_call_expression(&mut self, call_expression: &AstCallExpression) {
        for argument in &call_expression.arguments {
            self.visit_expression(argument);
        }
    }

    fn visit_assignment_expression(&mut self, assignment_expression: &AstAssignmentExpression) {
        self.visit_expression(&assignment_expression.expression);
    }

    fn visit_boolean_expression(&mut self, boolean: &AstBooleanExpression);

    fn visit_variable_expression(&mut self, variable_expression: &AstVariableExpression);

    fn visit_number_expression(&mut self, number: &AstNumberExpression);

    fn visit_error(&mut self, span: &TextSpan);

    fn visit_unary_expression(&mut self, unary_expression: &AstUnaryExpression);

    fn visit_binary_expression(&mut self, binary_expression: &AstBinaryExpression) {
        self.visit_expression(&binary_expression.left);
        self.visit_expression(&binary_expression.right);
    }

    fn visit_parenthesized_expression(
        &mut self,
        parenthesized_expression: &AstParenthesizedExpression,
    ) {
        self.visit_expression(&parenthesized_expression.expression);
    }
}
