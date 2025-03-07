//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::collections::HashMap;

use crate::ast::lexer::{TextSpan, Token};
use parser::Counter;
use printer::AstPrinter;
use termion::color::{Fg, Reset};
use visitor::AstVisitor;

pub mod evaluator;
pub mod lexer;
pub mod parser;
pub mod printer;
pub mod visitor;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct AstStatementId {
    pub id: usize,
}

impl AstStatementId {
    pub fn new(id: usize) -> Self {
        AstStatementId { id }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct AstExpressionId {
    pub id: usize,
}

impl AstExpressionId {
    pub fn new(id: usize) -> Self {
        AstExpressionId { id }
    }
}

pub struct AstNodeIdGenerator {
    pub next_statement_id: Counter,
    pub next_expression_id: Counter,
}

impl AstNodeIdGenerator {
    pub fn new() -> Self {
        Self {
            next_statement_id: Counter::new(),
            next_expression_id: Counter::new(),
        }
    }

    pub fn next_statement_id(&mut self) -> AstStatementId {
        let id = self.next_statement_id.get_value();
        self.next_statement_id.increment();
        AstStatementId::new(id)
    }

    pub fn next_expression_id(&mut self) -> AstExpressionId {
        let id = self.next_expression_id.get_value();
        self.next_expression_id.increment();
        AstExpressionId::new(id)
    }
}

pub struct Ast {
    pub statements: HashMap<AstStatementId, AstStatement>,
    pub expressions: HashMap<AstExpressionId, AstExpression>,
    pub node_id_generator: AstNodeIdGenerator,
}

impl Ast {
    pub fn new() -> Self {
        Self {
            statements: HashMap::new(),
            expressions: HashMap::new(),
            node_id_generator: AstNodeIdGenerator::new(),
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

#[derive(Debug, Clone)]
pub enum AstStatementKind {
    Expression(AstExpressionId),
    Let(AstLetStatement),
    If(AstIfStatement),
    Block(AstBlockStatement),
    While(AstWhileStatement),
    FuncDecl(AstFuncDeclStatement),
    Return(AstReturnStatement),
}

#[derive(Debug, Clone)]
pub struct AstReturnStatement {
    pub return_keyword: Token,
    pub return_value: Option<AstExpression>,
}

#[derive(Debug, Clone)]
pub struct AstFuncDeclParameter {
    pub identifier: Token,
}

#[derive(Debug, Clone)]
pub struct AstFuncDeclStatement {
    pub identifier: Token,
    pub parameters: Vec<AstFuncDeclParameter>,
    pub body: Box<AstStatement>,
}

#[derive(Debug, Clone)]
pub struct AstWhileStatement {
    pub while_keyword: Token,
    pub condition: AstExpression,
    pub body: Box<AstStatement>,
}

#[derive(Debug, Clone)]
pub struct AstBlockStatement {
    pub statements: Vec<AstStatement>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct AstIfStatement {
    pub if_keyword: Token,
    pub condition: AstExpression,
    pub then_branch: Box<AstStatement>,
    pub else_branch: Option<AstElseStatement>,
}

#[derive(Debug, Clone)]
pub struct AstLetStatement {
    pub identifier: Token,
    pub initializer: AstExpression,
}

#[derive(Debug, Clone)]
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
        AstStatement::new(AstStatementKind::Let(AstLetStatement {
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
        AstStatement::new(AstStatementKind::If(AstIfStatement {
            if_keyword,
            condition,
            then_branch: Box::new(then),
            else_branch: else_statement,
        }))
    }

    pub fn block_statement(statements: Vec<AstStatement>) -> Self {
        AstStatement::new(AstStatementKind::Block(AstBlockStatement { statements }))
    }

    pub fn while_statement(
        while_keyword: Token,
        condition: AstExpression,
        body: AstStatement,
    ) -> Self {
        AstStatement::new(AstStatementKind::While(AstWhileStatement {
            while_keyword,
            condition,
            body: Box::new(body),
        }))
    }

    pub fn return_statement(return_keyword: Token, return_value: Option<AstExpression>) -> Self {
        AstStatement::new(AstStatementKind::Return(AstReturnStatement {
            return_keyword,
            return_value,
        }))
    }

    pub fn func_decl_statement(
        identifier: Token,
        parameters: Vec<AstFuncDeclParameter>,
        body: AstStatement,
    ) -> Self {
        AstStatement::new(AstStatementKind::FuncDecl(AstFuncDeclStatement {
            identifier,
            parameters,
            body: Box::new(body),
        }))
    }
}

#[derive(Debug, Clone)]
pub enum AstExpressionKind {
    Number(AstNumberExpression),
    Binary(AstBinaryExpression),
    Unary(AstUnaryExpression),
    Parenthesized(AstParenthesizedExpression),
    Variable(AstVariableExpression),
    Assignment(AstAssignmentExpression),
    Boolean(AstBooleanExpression),
    Call(AstCallExpression),
    Error(TextSpan),
}

#[derive(Debug, Clone)]
pub struct AstCallExpression {
    pub identifier: Token,
    pub arguments: Vec<AstExpression>,
}

#[derive(Debug, Clone)]
pub struct AstBooleanExpression {
    pub value: bool,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct AstAssignmentExpression {
    pub identifier: Token,
    pub expression: Box<AstExpression>,
}

#[derive(Debug, Clone)]
pub enum AstUnaryOperatorKind {
    Minus,
    BitwiseNot,
}

#[derive(Debug, Clone)]
pub struct AstUnaryOperator {
    kind: AstUnaryOperatorKind,
    token: Token,
}

impl AstUnaryOperator {
    pub fn new(kind: AstUnaryOperatorKind, token: Token) -> Self {
        Self { kind, token }
    }
}

#[derive(Debug, Clone)]
pub struct AstUnaryExpression {
    pub operator: AstUnaryOperator,
    pub operand: Box<AstExpression>,
}

#[derive(Debug, Clone)]
pub struct AstVariableExpression {
    pub identifier: Token,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct AstBinaryExpression {
    left: Box<AstExpression>,
    operator: AstBinaryOperator,
    right: Box<AstExpression>,
}

#[derive(Debug, Clone)]
pub struct AstNumberExpression {
    number: i64,
}

#[derive(Debug, Clone)]
pub struct AstParenthesizedExpression {
    expression: Box<AstExpression>,
}

#[derive(Debug, Clone)]
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

    pub fn boolean(token: Token, value: bool) -> Self {
        AstExpression::new(AstExpressionKind::Boolean(AstBooleanExpression {
            token,
            value,
        }))
    }

    pub fn call(identifier: Token, arguments: Vec<AstExpression>) -> Self {
        AstExpression::new(AstExpressionKind::Call(AstCallExpression {
            identifier,
            arguments,
        }))
    }

    pub fn error(span: TextSpan) -> Self {
        AstExpression::new(AstExpressionKind::Error(span))
    }
}

#[cfg(test)]
mod test {
    use crate::compilation_unit::CompilationUnit;

    use super::{
        Ast, AstAssignmentExpression, AstBinaryExpression, AstBlockStatement, AstBooleanExpression,
        AstCallExpression, AstIfStatement, AstReturnStatement, AstUnaryExpression, AstVisitor,
        AstWhileStatement,
    };

    #[derive(Debug, PartialEq, Eq)]
    enum TestAstNode {
        Number(i64),
        Boolean(bool),
        Binary,
        Unary,
        Parenthesized,
        Let,
        Assignment,
        Block,
        Variable(String),
        If,
        Else,
        Func,
        While,
        Return,
        Call,
    }

    struct AstVerifier {
        expected: Vec<TestAstNode>,
        actual: Vec<TestAstNode>,
    }

    impl AstVerifier {
        pub fn new(input: &str, expected: Vec<TestAstNode>) -> Self {
            let compilation_unit = CompilationUnit::compile(input).expect("Failed to compile");
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

    impl AstVisitor<'_> for AstVerifier {
        fn visit_func_decl_statement(&mut self, func_decl_statement: &super::AstFuncDeclStatement) {
            self.actual.push(TestAstNode::Func);
            self.visit_statement(&func_decl_statement.body);
        }

        fn visit_return_statement(&mut self, return_statement: &AstReturnStatement) {
            self.actual.push(TestAstNode::Return);
            if let Some(expr) = &return_statement.return_value {
                self.visit_expression(expr);
            }
        }

        fn visit_let_statement(&mut self, let_statement: &super::AstLetStatement) {
            self.actual.push(TestAstNode::Let);
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

        fn visit_assignment_expression(&mut self, assignment_expression: &AstAssignmentExpression) {
            self.actual.push(TestAstNode::Assignment);
            self.visit_expression(&assignment_expression.expression);
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

        fn visit_binary_expression(&mut self, binary_expression: &AstBinaryExpression) {
            self.actual.push(TestAstNode::Binary);
            self.visit_expression(&binary_expression.left);
            self.visit_expression(&binary_expression.right);
        }

        fn visit_boolean_expression(&mut self, boolean: &AstBooleanExpression) {
            self.actual.push(TestAstNode::Boolean(boolean.value));
        }

        fn visit_if_statement(&mut self, if_statement: &AstIfStatement) {
            self.actual.push(TestAstNode::If);
            self.visit_expression(&if_statement.condition);
            self.visit_statement(&if_statement.then_branch);
            if let Some(else_branch) = &if_statement.else_branch {
                self.actual.push(TestAstNode::Else);

                self.visit_statement(&else_branch.else_statement);
            }
        }

        fn visit_while_statement(&mut self, while_statement: &AstWhileStatement) {
            self.actual.push(TestAstNode::While);
            self.visit_expression(&while_statement.condition);
            self.visit_statement(&while_statement.body);
        }

        fn visit_block_statement(&mut self, block_statement: &AstBlockStatement) {
            self.actual.push(TestAstNode::Block);
            for statement in &block_statement.statements {
                self.visit_statement(statement);
            }
        }

        fn visit_call_expression(&mut self, call_expression: &AstCallExpression) {
            self.actual.push(TestAstNode::Call);
            for argument in &call_expression.arguments {
                self.visit_expression(argument);
            }
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
            TestAstNode::Let,
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
            TestAstNode::Let,
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
            TestAstNode::Let,
            TestAstNode::Number(1),
            TestAstNode::Let,
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
            TestAstNode::Let,
            TestAstNode::Number(1),
            TestAstNode::Let,
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
            TestAstNode::Let,
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
            TestAstNode::Let,
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
            TestAstNode::Let,
            TestAstNode::Binary,
            TestAstNode::Number(1),
            TestAstNode::Number(2),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_not() {
        let input = "let a = ~1";
        let expected = vec![TestAstNode::Let, TestAstNode::Unary, TestAstNode::Number(1)];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_shift_left() {
        let input = "let a = 1 << 2";
        let expected = vec![
            TestAstNode::Let,
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
            TestAstNode::Let,
            TestAstNode::Binary,
            TestAstNode::Number(2),
            TestAstNode::Number(1),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_negation() {
        let input = "let a = -1";
        let expected = vec![TestAstNode::Let, TestAstNode::Unary, TestAstNode::Number(1)];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_power() {
        let input = "let a = 1 ** 2";
        let expected = vec![
            TestAstNode::Let,
            TestAstNode::Binary,
            TestAstNode::Number(1),
            TestAstNode::Number(2),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_if_statement() {
        let input = "\
        let a = 1
        if a > 0 {
            a = 20
        }
        ";
        let expected = vec![
            TestAstNode::Let,
            TestAstNode::Number(1),
            TestAstNode::If,
            TestAstNode::Binary,
            TestAstNode::Variable("a".to_string()),
            TestAstNode::Number(0),
            TestAstNode::Block,
            TestAstNode::Assignment,
            TestAstNode::Number(20),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_if_statement_with_else() {
        let input = "\
        let a = 1
        if a > 0 {
            a = 20
        } else {
            a = 30
        }
        ";
        let expected = vec![
            TestAstNode::Let,
            TestAstNode::Number(1),
            TestAstNode::If,
            TestAstNode::Binary,
            TestAstNode::Variable("a".to_string()),
            TestAstNode::Number(0),
            TestAstNode::Block,
            TestAstNode::Assignment,
            TestAstNode::Number(20),
            TestAstNode::Else,
            TestAstNode::Block,
            TestAstNode::Assignment,
            TestAstNode::Number(30),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_while_statement() {
        let input = "\
        let a = 1
        while a < 10 {
            a = a + 1
        }
        ";
        let expected = vec![
            TestAstNode::Let,
            TestAstNode::Number(1),
            TestAstNode::While,
            TestAstNode::Binary,
            TestAstNode::Variable("a".to_string()),
            TestAstNode::Number(10),
            TestAstNode::Block,
            TestAstNode::Assignment,
            TestAstNode::Binary,
            TestAstNode::Variable("a".to_string()),
            TestAstNode::Number(1),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_function_declaration() {
        let input = "\
        func add(a, b) {
            return a + b
        }
        ";
        let expected = vec![
            TestAstNode::Func,
            TestAstNode::Block,
            TestAstNode::Return,
            TestAstNode::Binary,
            TestAstNode::Variable("a".to_string()),
            TestAstNode::Variable("b".to_string()),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_call_expression() {
        let input = "\
        func add(a, b) {
            return a + b
        }
        add(2 * 3, 4 + 5)";
        let expected = vec![
            TestAstNode::Func,
            TestAstNode::Block,
            TestAstNode::Return,
            TestAstNode::Binary,
            TestAstNode::Variable("a".to_string()),
            TestAstNode::Variable("b".to_string()),
            TestAstNode::Call,
            TestAstNode::Binary,
            TestAstNode::Number(2),
            TestAstNode::Number(3),
            TestAstNode::Binary,
            TestAstNode::Number(4),
            TestAstNode::Number(5),
        ];

        assert_tree(input, expected);
    }
}
