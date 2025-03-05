//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use lexer::{TextSpan, Token};
use termion::color::{self, Fg, Reset};

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
        let mut printer = AstPrinter::new();
        self.visit(&mut printer);
        println!("{}", printer.result);
    }
}

pub trait AstVisitor {
    fn do_visit_statement(&mut self, statement: &AstStatement) {
        match &statement.kind {
            AstStatementKind::Expression(expr) => {
                self.visit_expression(expr);
            }
            AstStatementKind::LetStatement(expr) => {
                self.visit_let_statement(expr);
            }
            AstStatementKind::IfStatement(expr) => {
                self.visit_if_statement(expr);
            }
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

    fn visit_statement(&mut self, statement: &AstStatement) {
        self.do_visit_statement(statement);
    }

    fn do_visit_expression(&mut self, expression: &AstExpression) {
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
        }
    }

    fn visit_expression(&mut self, expression: &AstExpression) {
        self.do_visit_expression(expression);
    }

    fn visit_assignment_expression(&mut self, assignment_expression: &AstAssignmentExpression) {
        self.visit_expression(&assignment_expression.expression);
    }

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

pub struct AstPrinter {
    indent: usize,
    result: String,
}

impl AstPrinter {
    const NUMBER_COLOR: color::Magenta = color::Magenta;
    const TEXT_COLOR: color::LightWhite = color::LightWhite;
    const KEYWORD_COLOR: color::Blue = color::Blue;
    const VARIABLE_COLOR: color::Green = color::Green;

    fn add_whitespace(&mut self) {
        self.result.push(' ');
    }

    fn add_newline(&mut self) {
        self.result.push('\n');
    }

    fn add_keyword(&mut self, keyword: &str) {
        self.result
            .push_str(&format!("{}{}", Self::KEYWORD_COLOR.fg_str(), keyword));
    }

    fn add_text(&mut self, text: &str) {
        self.result
            .push_str(&format!("{}{}", Self::TEXT_COLOR.fg_str(), text));
    }

    fn add_variable(&mut self, variable: &str) {
        self.result
            .push_str(&format!("{}{}", Self::VARIABLE_COLOR.fg_str(), variable));
    }

    pub fn new() -> Self {
        Self {
            indent: 0,
            result: String::new(),
        }
    }
}

impl AstVisitor for AstPrinter {
    fn visit_if_statement(&mut self, if_statement: &AstIfStatement) {
        self.add_keyword("if");
        self.add_whitespace();
        self.visit_expression(&if_statement.condition);
        self.add_whitespace();
        self.visit_statement(&if_statement.then_branch);
        if let Some(else_branch) = &if_statement.else_branch {
            self.add_whitespace();
            self.add_keyword("else");
            self.add_whitespace();
            self.visit_statement(&else_branch.else_statement);
        }
    }

    fn visit_let_statement(&mut self, let_statement: &AstLetStatement) {
        self.add_keyword("let");
        self.add_whitespace();
        self.add_text(let_statement.identifier.span.literal.as_str());
        self.add_whitespace();
        self.add_text("=");
        self.add_whitespace();
        self.visit_expression(&let_statement.initializer);
    }

    fn visit_assignment_expression(&mut self, assignment_expression: &AstAssignmentExpression) {
        self.add_variable(assignment_expression.identifier.span.literal.as_str());
        self.add_whitespace();
        self.add_text("=");
        self.add_whitespace();
        self.visit_expression(&assignment_expression.expression);
    }

    fn visit_statement(&mut self, statement: &AstStatement) {
        Self::do_visit_statement(self, statement);
        self.result.push_str(&format!("{}\n", Fg(Reset)));
    }

    fn visit_number_expression(&mut self, number: &AstNumberExpression) {
        self.result
            .push_str(&format!("{}{}", Self::NUMBER_COLOR.fg_str(), number.number));
    }

    fn visit_error(&mut self, span: &TextSpan) {
        self.result
            .push_str(&format!("{}{}", Self::TEXT_COLOR.fg_str(), span.literal));
    }

    fn visit_unary_expression(&mut self, unary_expression: &AstUnaryExpression) {
        self.result.push_str(&format!(
            "{}{}",
            Self::TEXT_COLOR.fg_str(),
            unary_expression.operator.token.span.literal
        ));
        self.visit_expression(&unary_expression.operand);
    }

    fn visit_binary_expression(&mut self, binary_expression: &AstBinaryExpression) {
        self.visit_expression(&binary_expression.left);
        self.add_whitespace();
        self.result.push_str(&format!(
            "{}{}",
            Self::TEXT_COLOR.fg_str(),
            binary_expression.operator.token.span.literal
        ));
        self.add_whitespace();
        self.visit_expression(&binary_expression.right);
    }

    fn visit_parenthesized_expression(
        &mut self,
        parenthesized_expression: &AstParenthesizedExpression,
    ) {
        self.result
            .push_str(&format!("{}{}", Self::TEXT_COLOR.fg_str(), "("));
        self.visit_expression(&parenthesized_expression.expression);
        self.result
            .push_str(&format!("{}{}", Self::TEXT_COLOR.fg_str(), ")"));
    }

    fn visit_variable_expression(&mut self, variable_expression: &AstVariableExpression) {
        self.result.push_str(&format!(
            "{}{}",
            Self::VARIABLE_COLOR.fg_str(),
            variable_expression.identifier.span.literal
        ));
    }
}

pub enum AstStatementKind {
    Expression(AstExpression),
    LetStatement(AstLetStatement),
    IfStatement(AstIfStatement),
}

pub struct AstElseStatement {
    pub else_keyword: Token,
    pub else_statement: Box<AstStatement>,
}

impl AstElseStatement {
    pub fn new(else_keyword: Token, else_statement: AstStatement) -> Self {
        AstElseStatement {
            else_keyword,
            else_statement: Box::new(else_statement),
        }
    }
}

pub struct AstIfStatement {
    pub if_keyword: Token,
    pub condition: AstExpression,
    pub then_branch: Box<AstStatement>,
    pub else_branch: Option<AstElseStatement>,
}

pub struct AstLetStatement {
    pub identifier: Token,
    pub initializer: AstExpression,
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

    pub fn let_statement(identifier: Token, initializer: AstExpression) -> Self {
        AstStatement::new(AstStatementKind::LetStatement(AstLetStatement {
            identifier,
            initializer,
        }))
    }

    pub fn if_statement(
        if_keyword: Token,
        condition: AstExpression,
        then: AstStatement,
        else_statement: Option<AstElseStatement>,
    ) -> Self {
        AstStatement::new(AstStatementKind::IfStatement(AstIfStatement {
            if_keyword,
            condition,
            then_branch: Box::new(then),
            else_branch: else_statement,
        }))
    }
}

pub enum AstExpressionKind {
    Number(AstNumberExpression),
    Binary(AstBinaryExpression),
    Unary(AstUnaryExpression),
    Parenthesized(AstParenthesizedExpression),
    Variable(AstVariableExpression),
    Assignment(AstAssignmentExpression),
    Error(TextSpan),
}

pub struct AstAssignmentExpression {
    pub identifier: Token,
    pub expression: Box<AstExpression>,
}

pub enum AstUnaryOperatorKind {
    Minus,
    BitwiseNot,
}

pub struct AstUnaryOperator {
    kind: AstUnaryOperatorKind,
    token: Token,
}

impl AstUnaryOperator {
    pub fn new(kind: AstUnaryOperatorKind, token: Token) -> Self {
        Self { kind, token }
    }
}

pub struct AstUnaryExpression {
    pub operator: AstUnaryOperator,
    pub operand: Box<AstExpression>,
}

#[derive(Debug)]
pub struct AstVariableExpression {
    pub identifier: Token,
}

#[derive(Debug)]
pub enum AstBinaryOperatorKind {
    Plus,
    Minus,
    Multiply,
    Divide,
    Power,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
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
            AstBinaryOperatorKind::Equals => 30,
            AstBinaryOperatorKind::NotEquals => 30,
            AstBinaryOperatorKind::LessThan => 29,
            AstBinaryOperatorKind::LessThanOrEqual => 29,
            AstBinaryOperatorKind::GreaterThan => 29,
            AstBinaryOperatorKind::GreaterThanOrEqual => 29,
            AstBinaryOperatorKind::Power => 20,
            AstBinaryOperatorKind::Multiply => 19,
            AstBinaryOperatorKind::Divide => 19,
            AstBinaryOperatorKind::Plus => 18,
            AstBinaryOperatorKind::Minus => 18,
            AstBinaryOperatorKind::LeftShift => 17,
            AstBinaryOperatorKind::RightShift => 17,
            AstBinaryOperatorKind::BitwiseAnd => 16,
            AstBinaryOperatorKind::BitwiseXor => 15,
            AstBinaryOperatorKind::BitwiseOr => 14,
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

    pub fn unary(operator: AstUnaryOperator, operand: AstExpression) -> Self {
        AstExpression::new(AstExpressionKind::Unary(AstUnaryExpression {
            operator,
            operand: Box::new(operand),
        }))
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

    pub fn identifier(identifier: Token) -> Self {
        AstExpression::new(AstExpressionKind::Variable(AstVariableExpression {
            identifier,
        }))
    }

    pub fn assignment(identifier: Token, expression: AstExpression) -> Self {
        AstExpression::new(AstExpressionKind::Assignment(AstAssignmentExpression {
            identifier,
            expression: Box::new(expression),
        }))
    }

    pub fn error(span: TextSpan) -> Self {
        AstExpression::new(AstExpressionKind::Error(span))
    }
}

#[cfg(test)]
mod test {
    use crate::compilation_unit::CompilationUnit;

    use super::{Ast, AstUnaryExpression, AstVisitor};

    #[derive(Debug, PartialEq, Eq)]
    enum TestAstNode {
        Number(i64),
        Binary,
        Unary,
        Parenthesized,
        LetStmt,
        Variable(String),
    }

    struct AstVerifier {
        expected: Vec<TestAstNode>,
        actual: Vec<TestAstNode>,
    }

    impl AstVerifier {
        pub fn new(input: &str, expected: Vec<TestAstNode>) -> Self {
            let compilation_unit = CompilationUnit::compile(input);
            assert_eq!(
                compilation_unit.diagnostics_list.borrow().diagnostics.len(),
                0,
                "Expected no diagnostics, but got {:?}",
                compilation_unit.diagnostics_list.borrow().diagnostics
            );
            let mut verifier = AstVerifier {
                expected,
                actual: Vec::new(),
            };
            verifier.flatten_ast(&compilation_unit.ast);
            verifier
        }

        fn flatten_ast(&mut self, ast: &Ast) {
            self.actual.clear();
            ast.visit(&mut *self);
        }

        pub fn verify(&self) {
            assert_eq!(
                self.expected.len(),
                self.actual.len(),
                "Expected {} nodes, but got {}",
                self.expected.len(),
                self.actual.len()
            );

            for (index, (expected, actual)) in
                self.expected.iter().zip(self.actual.iter()).enumerate()
            {
                assert_eq!(
                    expected, actual,
                    "Expected {:?} at index {}, but got {:?}",
                    expected, index, actual
                );
            }
        }
    }

    impl AstVisitor for AstVerifier {
        fn visit_let_statement(&mut self, let_statement: &super::AstLetStatement) {
            self.actual.push(TestAstNode::LetStmt);
            self.visit_expression(&let_statement.initializer);
        }

        fn visit_variable_expression(
            &mut self,
            variable_expression: &super::AstVariableExpression,
        ) {
            self.actual.push(TestAstNode::Variable(
                variable_expression.identifier.span.literal.clone(),
            ));
        }

        fn visit_number_expression(&mut self, number: &super::AstNumberExpression) {
            self.actual.push(TestAstNode::Number(number.number));
        }

        fn visit_error(&mut self, span: &super::lexer::TextSpan) {
            // TODO
        }

        fn visit_unary_expression(&mut self, unary_expression: &AstUnaryExpression) {
            self.actual.push(TestAstNode::Unary);
            self.visit_expression(&unary_expression.operand);
        }

        fn visit_parenthesized_expression(
            &mut self,
            parenthesized_expression: &super::AstParenthesizedExpression,
        ) {
            self.actual.push(TestAstNode::Parenthesized);
            self.visit_expression(&parenthesized_expression.expression);
        }

        fn visit_binary_expression(&mut self, binary_expression: &super::AstBinaryExpression) {
            self.actual.push(TestAstNode::Binary);
            self.visit_expression(&binary_expression.left);
            self.visit_expression(&binary_expression.right);
        }
    }

    fn assert_tree(input: &str, expected: Vec<TestAstNode>) {
        let verifier = AstVerifier::new(input, expected);
        verifier.verify();
    }

    #[test]
    pub fn should_parse_basic_binary_expression() {
        let input = "let a = 1 + 2";
        let expected = vec![
            TestAstNode::LetStmt,
            TestAstNode::Binary,
            TestAstNode::Number(1),
            TestAstNode::Number(2),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_parenthesized_binary_expression() {
        let input = "let a = (1 + 2) * 3";
        let expected = vec![
            TestAstNode::LetStmt,
            TestAstNode::Binary,
            TestAstNode::Parenthesized,
            TestAstNode::Binary,
            TestAstNode::Number(1),
            TestAstNode::Number(2),
            TestAstNode::Number(3),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_parenthesized_binary_expression_with_variable() {
        let input = "\
        let b = 1
        let a = (1 + 2) * b";
        let expected = vec![
            TestAstNode::LetStmt,
            TestAstNode::Number(1),
            TestAstNode::LetStmt,
            TestAstNode::Binary,
            TestAstNode::Parenthesized,
            TestAstNode::Binary,
            TestAstNode::Number(1),
            TestAstNode::Number(2),
            TestAstNode::Variable("b".to_string()),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_parenthesized_binary_expression_with_variable_and_number() {
        let input = "\
        let b = 1
        let a = (1 + 2) * b + 3";
        let expected = vec![
            TestAstNode::LetStmt,
            TestAstNode::Number(1),
            TestAstNode::LetStmt,
            TestAstNode::Binary,
            TestAstNode::Binary,
            TestAstNode::Parenthesized,
            TestAstNode::Binary,
            TestAstNode::Number(1),
            TestAstNode::Number(2),
            TestAstNode::Variable("b".to_string()),
            TestAstNode::Number(3),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_and() {
        let input = "let a = 1 & 2";
        let expected = vec![
            TestAstNode::LetStmt,
            TestAstNode::Binary,
            TestAstNode::Number(1),
            TestAstNode::Number(2),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_or() {
        let input = "let a = 1 | 2";
        let expected = vec![
            TestAstNode::LetStmt,
            TestAstNode::Binary,
            TestAstNode::Number(1),
            TestAstNode::Number(2),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_xor() {
        let input = "let a = 1 ^ 2";
        let expected = vec![
            TestAstNode::LetStmt,
            TestAstNode::Binary,
            TestAstNode::Number(1),
            TestAstNode::Number(2),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_not() {
        let input = "let a = ~1";
        let expected = vec![
            TestAstNode::LetStmt,
            TestAstNode::Unary,
            TestAstNode::Number(1),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_shift_left() {
        let input = "let a = 1 << 2";
        let expected = vec![
            TestAstNode::LetStmt,
            TestAstNode::Binary,
            TestAstNode::Number(1),
            TestAstNode::Number(2),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_shift_right() {
        let input = "let a = 2 >> 1";
        let expected = vec![
            TestAstNode::LetStmt,
            TestAstNode::Binary,
            TestAstNode::Number(2),
            TestAstNode::Number(1),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_negation() {
        let input = "let a = -1";
        let expected = vec![
            TestAstNode::LetStmt,
            TestAstNode::Unary,
            TestAstNode::Number(1),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_power() {
        let input = "let a = 1 ** 2";
        let expected = vec![
            TestAstNode::LetStmt,
            TestAstNode::Binary,
            TestAstNode::Number(1),
            TestAstNode::Number(2),
        ];

        assert_tree(input, expected);
    }
}
