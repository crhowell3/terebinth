//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::collections::HashMap;
use std::hash::Hash;

use crate::ast::parser::Counter;
use crate::typings::Type;
use crate::{ast::lexer::Token, source::span::TextSpan};
use printer::AstPrinter;
use termion::color::{Fg, Reset};
use visitor::AstVisitor;

pub mod evaluator;
pub mod lexer;
pub mod parser;
pub mod printer;
pub mod visitor;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct AstStmtId {
    pub id: usize,
}

impl AstStmtId {
    pub fn new(id: usize) -> Self {
        AstStmtId { id }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct AstExprId {
    pub id: usize,
}

impl AstExprId {
    pub fn new(id: usize) -> Self {
        AstExprId { id }
    }
}

#[derive(Debug, Clone)]
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

    pub fn next_statement_id(&self) -> AstStmtId {
        let id = self.next_statement_id.get_value();
        self.next_statement_id.increment();
        AstStmtId::new(id)
    }

    pub fn next_expression_id(&self) -> AstExprId {
        let id = self.next_expression_id.get_value();
        self.next_expression_id.increment();
        AstExprId::new(id)
    }
}

#[derive(Debug, Clone)]
pub struct Ast {
    pub statements: HashMap<AstStmtId, AstStatement>,
    pub expressions: HashMap<AstExprId, AstExpression>,
    pub top_level_statements: Vec<AstStmtId>,
    pub node_id_generator: AstNodeIdGenerator,
}

impl Ast {
    pub fn new() -> Self {
        Self {
            statements: HashMap::new(),
            expressions: HashMap::new(),
            top_level_statements: Vec::new(),
            node_id_generator: AstNodeIdGenerator::new(),
        }
    }

    pub fn query_expr(&self, expr_id: AstExprId) -> &AstExpression {
        &self.expressions[&expr_id]
    }

    pub fn query_stmt(&self, stmt_id: AstStmtId) -> &AstStatement {
        &self.statements[&stmt_id]
    }

    pub fn set_type(&mut self, expr_id: AstExprId, expr_type: Type) {
        let expr = self.expressions.get_mut(&expr_id).unwrap();
        expr.expr_type = expr_type;
    }

    pub fn mark_top_level_statement(&mut self, stmt_id: AstStmtId) {
        self.top_level_statements.push(stmt_id);
    }

    fn stmt_from_kind(&mut self, kind: AstStatementKind) -> &AstStatement {
        let stmt = AstStatement::new(kind, self.node_id_generator.next_statement_id());
        let id = stmt.id;
        self.statements.insert(id, stmt);
        &self.statements[&id]
    }

    pub fn expression_statement(&mut self, expr_id: AstExprId) -> &AstStatement {
        self.stmt_from_kind(AstStatementKind::Expression(expr_id))
    }

    pub fn let_statement(
        &mut self,
        identifier: Token,
        initializer: AstExprId,
        type_annotation: Option<StaticTypeAnnotation>,
    ) -> &AstStatement {
        self.stmt_from_kind(AstStatementKind::Let(AstLetStatement {
            identifier,
            initializer,
            type_annotation,
        }))
    }

    pub fn if_statement(
        &mut self,
        if_keyword: Token,
        condition: AstExprId,
        then: AstStmtId,
        else_statement: Option<AstElseStatement>,
    ) -> &AstStatement {
        self.stmt_from_kind(AstStatementKind::If(AstIfStatement {
            if_keyword,
            condition,
            then_branch: then,
            else_branch: else_statement,
        }))
    }

    pub fn block_statement(&mut self, statements: Vec<AstStmtId>) -> &AstStatement {
        self.stmt_from_kind(AstStatementKind::Block(AstBlockStatement { statements }))
    }

    pub fn while_statement(
        &mut self,
        while_keyword: Token,
        condition: AstExprId,
        body: AstStmtId,
    ) -> &AstStatement {
        self.stmt_from_kind(AstStatementKind::While(AstWhileStatement {
            while_keyword,
            condition,
            body,
        }))
    }

    pub fn return_statement(
        &mut self,
        return_keyword: Token,
        return_value: Option<AstExprId>,
    ) -> &AstStatement {
        self.stmt_from_kind(AstStatementKind::Return(AstReturnStatement {
            return_keyword,
            return_value,
        }))
    }

    pub fn func_decl_statement(
        &mut self,
        identifier: Token,
        parameters: Vec<AstFuncDeclParameter>,
        body: AstStmtId,
        return_type: Option<AstFunctionReturnType>,
    ) -> &AstStatement {
        self.stmt_from_kind(AstStatementKind::FuncDecl(AstFuncDeclStatement {
            identifier,
            parameters,
            body,
            return_type,
        }))
    }

    fn expr_from_kind(&mut self, kind: AstExpressionKind) -> &AstExpression {
        let expr = AstExpression::new(
            kind,
            self.node_id_generator.next_expression_id(),
            Type::Unresolved,
        );
        let expr_id = expr.id;
        self.expressions.insert(expr_id, expr);
        &self.expressions[&expr_id]
    }

    pub fn number_expression(&mut self, token: Token, number: i64) -> &AstExpression {
        self.expr_from_kind(AstExpressionKind::Number(AstNumberExpression {
            number,
            token,
        }))
    }

    pub fn binary_expression(
        &mut self,
        operator: AstBinaryOperator,
        left: AstExprId,
        right: AstExprId,
    ) -> &AstExpression {
        self.expr_from_kind(AstExpressionKind::Binary(AstBinaryExpression {
            left,
            operator,
            right,
        }))
    }

    pub fn parenthesized_expression(
        &mut self,
        left_paren: Token,
        expression: AstExprId,
        right_paren: Token,
    ) -> &AstExpression {
        self.expr_from_kind(AstExpressionKind::Parenthesized(
            AstParenthesizedExpression {
                left_paren,
                expression,
                right_paren,
            },
        ))
    }

    pub fn variable_expression(&mut self, identifier: Token) -> &AstExpression {
        self.expr_from_kind(AstExpressionKind::Variable(AstVariableExpression {
            identifier,
        }))
    }

    pub fn unary_expression(
        &mut self,
        operator: AstUnaryOperator,
        operand: AstExprId,
    ) -> &AstExpression {
        self.expr_from_kind(AstExpressionKind::Unary(AstUnaryExpression {
            operator,
            operand,
        }))
    }

    pub fn assignment_expression(
        &mut self,
        identifier: Token,
        equals: Token,
        expression: AstExprId,
    ) -> &AstExpression {
        self.expr_from_kind(AstExpressionKind::Assignment(AstAssignmentExpression {
            identifier,
            equals,
            expression,
        }))
    }

    pub fn boolean_expression(&mut self, token: Token, value: bool) -> &AstExpression {
        self.expr_from_kind(AstExpressionKind::Boolean(AstBooleanExpression {
            value,
            token,
        }))
    }

    pub fn call_expression(
        &mut self,
        identifier: Token,
        left_paren: Token,
        arguments: Vec<AstExprId>,
        right_paren: Token,
    ) -> &AstExpression {
        self.expr_from_kind(AstExpressionKind::Call(AstCallExpression {
            identifier,
            left_paren,
            arguments,
            right_paren,
        }))
    }

    pub fn error_expression(&mut self, span: TextSpan) -> &AstExpression {
        self.expr_from_kind(AstExpressionKind::Error(span))
    }

    pub fn visit(&self, visitor: &mut dyn AstVisitor) {
        for statement in &self.top_level_statements {
            visitor.visit_statement(*statement);
        }
    }

    pub fn visualize(&self) {
        let mut printer = AstPrinter::new(self);
        self.visit(&mut printer);
        println!("{}", printer.result);
    }
}

#[derive(Debug, Clone)]
pub enum AstStatementKind {
    Expression(AstExprId),
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
    pub return_value: Option<AstExprId>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StaticTypeAnnotation {
    pub colon: Token,
    pub type_name: Token,
}

impl StaticTypeAnnotation {
    pub fn new(colon: Token, type_name: Token) -> Self {
        Self { colon, type_name }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AstFunctionReturnType {
    pub arrow: Token,
    pub type_name: Token,
}

impl AstFunctionReturnType {
    pub fn new(arrow: Token, type_name: Token) -> Self {
        Self { arrow, type_name }
    }
}

#[derive(Debug, Clone)]
pub struct AstFuncDeclParameter {
    pub identifier: Token,
    pub type_annotation: StaticTypeAnnotation,
}

#[derive(Debug, Clone)]
pub struct AstFuncDeclStatement {
    pub identifier: Token,
    pub parameters: Vec<AstFuncDeclParameter>,
    pub body: AstStmtId,
    pub return_type: Option<AstFunctionReturnType>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AstWhileStatement {
    pub while_keyword: Token,
    pub condition: AstExprId,
    pub body: AstStmtId,
}

#[derive(Debug, Clone)]
pub struct AstBlockStatement {
    pub statements: Vec<AstStmtId>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AstElseStatement {
    pub else_keyword: Token,
    pub else_statement: AstStmtId,
}

impl AstElseStatement {
    pub fn new(else_keyword: Token, else_statement: AstStmtId) -> Self {
        AstElseStatement {
            else_keyword,
            else_statement,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AstIfStatement {
    pub if_keyword: Token,
    pub condition: AstExprId,
    pub then_branch: AstStmtId,
    pub else_branch: Option<AstElseStatement>,
}

#[derive(Debug, Clone)]
pub struct AstLetStatement {
    pub identifier: Token,
    pub initializer: AstExprId,
    pub type_annotation: Option<StaticTypeAnnotation>,
}

#[derive(Debug, Clone)]
pub struct AstStatement {
    kind: AstStatementKind,
    id: AstStmtId,
}

impl AstStatement {
    pub fn new(kind: AstStatementKind, id: AstStmtId) -> Self {
        AstStatement { kind, id }
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
    pub left_paren: Token,
    pub arguments: Vec<AstExprId>,
    pub right_paren: Token,
}

#[derive(Debug, Clone)]
pub struct AstBooleanExpression {
    pub value: bool,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct AstAssignmentExpression {
    pub identifier: Token,
    pub equals: Token,
    pub expression: AstExprId,
}

#[derive(Debug, Clone)]
pub enum AstUnaryOperatorKind {
    Minus,
    BitwiseNot,
}

#[derive(Debug, Clone)]
pub struct AstUnaryOperator {
    pub(crate) kind: AstUnaryOperatorKind,
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
    pub operand: AstExprId,
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

#[derive(Debug, Clone, PartialEq)]
pub enum AstBinaryOperatorAssociativity {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct AstBinaryOperator {
    pub kind: AstBinaryOperatorKind,
    pub token: Token,
}

impl AstBinaryOperator {
    pub fn new(kind: AstBinaryOperatorKind, token: Token) -> Self {
        AstBinaryOperator { kind, token }
    }

    pub fn precedence(&self) -> u8 {
        match self.kind {
            AstBinaryOperatorKind::Equals | AstBinaryOperatorKind::NotEquals => 30,
            AstBinaryOperatorKind::LessThan
            | AstBinaryOperatorKind::LessThanOrEqual
            | AstBinaryOperatorKind::GreaterThan
            | AstBinaryOperatorKind::GreaterThanOrEqual => 29,
            AstBinaryOperatorKind::Power => 20,
            AstBinaryOperatorKind::Multiply | AstBinaryOperatorKind::Divide => 19,
            AstBinaryOperatorKind::Plus | AstBinaryOperatorKind::Minus => 18,
            AstBinaryOperatorKind::LeftShift | AstBinaryOperatorKind::RightShift => 17,
            AstBinaryOperatorKind::BitwiseAnd => 16,
            AstBinaryOperatorKind::BitwiseXor => 15,
            AstBinaryOperatorKind::BitwiseOr => 14,
        }
    }

    pub fn associativity(&self) -> AstBinaryOperatorAssociativity {
        match self.kind {
            AstBinaryOperatorKind::Power => AstBinaryOperatorAssociativity::Right,
            _ => AstBinaryOperatorAssociativity::Left,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AstBinaryExpression {
    pub left: AstExprId,
    pub operator: AstBinaryOperator,
    pub right: AstExprId,
}

#[derive(Debug, Clone)]
pub struct AstNumberExpression {
    pub number: i64,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct AstParenthesizedExpression {
    pub left_paren: Token,
    pub expression: AstExprId,
    pub right_paren: Token,
}

#[derive(Debug, Clone)]
pub struct AstExpression {
    pub kind: AstExpressionKind,
    pub id: AstExprId,
    pub expr_type: Type,
}

impl AstExpression {
    pub fn new(kind: AstExpressionKind, id: AstExprId, expr_type: Type) -> Self {
        AstExpression {
            kind,
            id,
            expr_type,
        }
    }

    pub fn span(&self, ast: &Ast) -> TextSpan {
        match &self.kind {
            AstExpressionKind::Number(expr) => expr.token.span.clone(),
            AstExpressionKind::Binary(expr) => {
                let left = ast.query_expr(expr.left).span(ast);
                let operator = expr.operator.token.span.clone();
                let right = ast.query_expr(expr.right).span(ast);
                TextSpan::combine(vec![left, operator, right])
            }
            AstExpressionKind::Unary(expr) => {
                let operator = expr.operator.token.span.clone();
                let operand = ast.query_expr(expr.operand).span(ast);
                TextSpan::combine(vec![operator, operand])
            }
            AstExpressionKind::Parenthesized(expr) => {
                let open_paren = expr.left_paren.span.clone();
                let expression = ast.query_expr(expr.expression).span(ast);
                let close_paren = expr.right_paren.span.clone();
                TextSpan::combine(vec![open_paren, expression, close_paren])
            }
            AstExpressionKind::Variable(expr) => expr.identifier.span.clone(),
            AstExpressionKind::Assignment(expr) => {
                let identifier = expr.identifier.span.clone();
                let equals = expr.equals.span.clone();
                let expression = ast.query_expr(expr.expression).span(ast);
                TextSpan::combine(vec![identifier, equals, expression])
            }
            AstExpressionKind::Boolean(expr) => expr.token.span.clone(),
            AstExpressionKind::Call(expr) => {
                let identifier = expr.identifier.span.clone();
                let left_paren = expr.left_paren.span.clone();
                let right_paren = expr.right_paren.span.clone();
                let mut spans = vec![identifier, left_paren, right_paren];
                for arg in &expr.arguments {
                    spans.push(ast.query_expr(*arg).span(ast));
                }
                TextSpan::combine(spans)
            }
            AstExpressionKind::Error(span) => span.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ast::visitor::AstVisitor;
    use crate::compilation_unit::CompilationUnit;
    use crate::source::span::TextSpan;

    use super::{
        Ast, AstAssignmentExpression, AstBinaryExpression, AstBlockStatement, AstBooleanExpression,
        AstCallExpression, AstExpression, AstIfStatement, AstReturnStatement, AstUnaryExpression,
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
        ast: Ast,
    }

    impl AstVerifier {
        pub fn new(input: &str, expected: Vec<TestAstNode>) -> Self {
            let compilation_unit = CompilationUnit::compile(input).expect("Failed to compile");
            let mut verifier = AstVerifier {
                expected,
                actual: Vec::new(),
                ast: compilation_unit.ast,
            };
            verifier.flatten_ast();
            verifier
        }

        fn flatten_ast(&mut self) {
            self.actual.clear();
            let ast = &self.ast.clone();
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
        fn get_ast(&self) -> &Ast {
            &self.ast
        }

        fn visit_func_decl_statement(&mut self, func_decl_statement: &super::AstFuncDeclStatement) {
            self.actual.push(TestAstNode::Func);
            self.visit_statement(func_decl_statement.body);
        }

        fn visit_return_statement(&mut self, return_statement: &AstReturnStatement) {
            self.actual.push(TestAstNode::Return);
            if let Some(expr) = &return_statement.return_value {
                self.visit_expression(*expr);
            }
        }

        fn visit_let_statement(&mut self, let_statement: &super::AstLetStatement) {
            self.actual.push(TestAstNode::Let);
            self.visit_expression(let_statement.initializer);
        }

        fn visit_variable_expression(
            &mut self,
            variable_expression: &super::AstVariableExpression,
            _expr: &AstExpression,
        ) {
            self.actual.push(TestAstNode::Variable(
                variable_expression.identifier.span.literal.clone(),
            ));
        }

        fn visit_assignment_expression(
            &mut self,
            assignment_expression: &AstAssignmentExpression,
            _expr: &AstExpression,
        ) {
            self.actual.push(TestAstNode::Assignment);
            self.visit_expression(assignment_expression.expression);
        }

        fn visit_number_expression(
            &mut self,
            number: &super::AstNumberExpression,
            _expr: &AstExpression,
        ) {
            self.actual.push(TestAstNode::Number(number.number));
        }

        fn visit_error(&mut self, _span: &TextSpan) {
            // TODO
        }

        fn visit_unary_expression(
            &mut self,
            unary_expression: &AstUnaryExpression,
            _expr: &AstExpression,
        ) {
            self.actual.push(TestAstNode::Unary);
            self.visit_expression(unary_expression.operand);
        }

        fn visit_parenthesized_expression(
            &mut self,
            parenthesized_expression: &super::AstParenthesizedExpression,
            _expr: &AstExpression,
        ) {
            self.actual.push(TestAstNode::Parenthesized);
            self.visit_expression(parenthesized_expression.expression);
        }

        fn visit_binary_expression(
            &mut self,
            binary_expression: &AstBinaryExpression,
            _expr: &AstExpression,
        ) {
            self.actual.push(TestAstNode::Binary);
            self.visit_expression(binary_expression.left);
            self.visit_expression(binary_expression.right);
        }

        fn visit_boolean_expression(
            &mut self,
            boolean: &AstBooleanExpression,
            _expr: &AstExpression,
        ) {
            self.actual.push(TestAstNode::Boolean(boolean.value));
        }

        fn visit_if_statement(&mut self, if_statement: &AstIfStatement) {
            self.actual.push(TestAstNode::If);
            self.visit_expression(if_statement.condition);
            self.visit_statement(if_statement.then_branch);
            if let Some(else_branch) = &if_statement.else_branch {
                self.actual.push(TestAstNode::Else);

                self.visit_statement(else_branch.else_statement);
            }
        }

        fn visit_while_statement(&mut self, while_statement: &AstWhileStatement) {
            self.actual.push(TestAstNode::While);
            self.visit_expression(while_statement.condition);
            self.visit_statement(while_statement.body);
        }

        fn visit_block_statement(&mut self, block_statement: &AstBlockStatement) {
            self.actual.push(TestAstNode::Block);
            for statement in &block_statement.statements {
                self.visit_statement(*statement);
            }
        }

        fn visit_call_expression(
            &mut self,
            call_expression: &AstCallExpression,
            _expr: &AstExpression,
        ) {
            self.actual.push(TestAstNode::Call);
            for argument in &call_expression.arguments {
                self.visit_expression(*argument);
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
        func add(a: int, b: int) -> int {
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
        func add(a: int, b: int) -> int {
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
