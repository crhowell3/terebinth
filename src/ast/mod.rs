//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use lexer::Token;

pub mod evaluator;
pub mod lexer;
pub mod parser;

pub struct Ast {
    pub statements: Vec<AstStatement>,
}

impl Ast {
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }

    pub fn add_statement(&mut self, statement: AstStatement) {
        self.statements.push(statement);
    }

    pub fn visit(&self, visitor: &mut dyn AstVisitor) {
        for statement in &self.statements {
            visitor.visit_statement(statement);
        }
    }

    pub fn visualize(&self) {
        let mut printer = AstPrinter { indent: 0 };
        self.visit(&mut printer);
    }
}

pub trait AstVisitor {
    fn do_visit_statement(&mut self, statement: &AstStatement) {
        match &statement.kind {
            AstStatementKind::Expression(expr) => {
                self.visit_expression(expr);
            }
        }
    }

    fn visit_statement(&mut self, statement: &AstStatement) {
        self.do_visit_statement(statement);
    }

    fn do_visit_expression(&mut self, expression: &AstExpression) {
        match &expression.kind {
            AstExpressionKind::Number(number) => {
                self.visit_number(number);
            }
            AstExpressionKind::Binary(expr) => {
                self.visit_binary_expression(expr);
            }
            AstExpressionKind::Parenthesized(expr) => {
                self.visit_parenthesized_expression(expr);
            }
        }
    }

    fn visit_expression(&mut self, expression: &AstExpression) {
        self.do_visit_expression(expression);
    }

    fn visit_number(&mut self, number: &AstNumberExpression);

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

pub struct AstPrinter {
    indent: usize,
}
const LEVEL_INDENT: usize = 2;

impl AstVisitor for AstPrinter {
    fn visit_statement(&mut self, statement: &AstStatement) {
        self.print_with_indent("Statement:");
        self.indent += LEVEL_INDENT;
        AstVisitor::do_visit_statement(self, statement);
        self.indent -= LEVEL_INDENT;
    }

    fn visit_expression(&mut self, expression: &AstExpression) {
        self.print_with_indent("Expression:");
        self.indent += LEVEL_INDENT;
        AstVisitor::do_visit_expression(self, expression);
        self.indent -= 2;
    }

    fn visit_number(&mut self, number: &AstNumberExpression) {
        self.print_with_indent(&format!("Number: {}", number.number));
    }

    fn visit_binary_expression(&mut self, binary_expression: &AstBinaryExpression) {
        self.print_with_indent("Binary Expression:");
        self.indent += LEVEL_INDENT;
        self.print_with_indent(&format!("Operator: {:?}", binary_expression.operator.kind));
        self.visit_expression(&binary_expression.left);
        self.visit_expression(&binary_expression.right);
        self.indent -= LEVEL_INDENT;
    }

    fn visit_parenthesized_expression(
        &mut self,
        parenthesized_expression: &AstParenthesizedExpression,
    ) {
        self.print_with_indent("Parenthesized Expression:");
        self.indent += LEVEL_INDENT;
        self.visit_expression(&parenthesized_expression.expression);
        self.indent -= LEVEL_INDENT;
    }
}

impl AstPrinter {
    fn print_with_indent(&mut self, text: &str) {
        println!("{}{}", " ".repeat(self.indent), text);
    }
}

pub enum AstStatementKind {
    Expression(AstExpression),
}

pub struct AstStatement {
    kind: AstStatementKind,
}

impl AstStatement {
    pub fn new(kind: AstStatementKind) -> Self {
        AstStatement { kind }
    }

    pub fn expression(expr: AstExpression) -> Self {
        AstStatement::new(AstStatementKind::Expression(expr))
    }
}

pub enum AstExpressionKind {
    Number(AstNumberExpression),
    Binary(AstBinaryExpression),
    Parenthesized(AstParenthesizedExpression),
}

#[derive(Debug)]
pub enum AstBinaryOperatorKind {
    Plus,
    Minus,
    Multiply,
    Divide,
}

pub struct AstBinaryOperator {
    kind: AstBinaryOperatorKind,
    token: Token,
}

impl AstBinaryOperator {
    pub fn new(kind: AstBinaryOperatorKind, token: Token) -> Self {
        AstBinaryOperator { kind, token }
    }

    pub fn precedence(&self) -> u8 {
        match self.kind {
            AstBinaryOperatorKind::Plus => 1,
            AstBinaryOperatorKind::Minus => 1,
            AstBinaryOperatorKind::Multiply => 2,
            AstBinaryOperatorKind::Divide => 2,
        }
    }
}

pub struct AstBinaryExpression {
    left: Box<AstExpression>,
    operator: AstBinaryOperator,
    right: Box<AstExpression>,
}

pub struct AstNumberExpression {
    number: i64,
}

pub struct AstParenthesizedExpression {
    expression: Box<AstExpression>,
}

pub struct AstExpression {
    kind: AstExpressionKind,
}

impl AstExpression {
    pub fn new(kind: AstExpressionKind) -> Self {
        AstExpression { kind }
    }

    pub fn number(number: i64) -> Self {
        AstExpression::new(AstExpressionKind::Number(AstNumberExpression { number }))
    }

    pub fn binary(operator: AstBinaryOperator, left: AstExpression, right: AstExpression) -> Self {
        AstExpression::new(AstExpressionKind::Binary(AstBinaryExpression {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        }))
    }

    pub fn parenthesized(expression: AstExpression) -> Self {
        AstExpression::new(AstExpressionKind::Parenthesized(
            AstParenthesizedExpression {
                expression: Box::new(expression),
            },
        ))
    }
}
