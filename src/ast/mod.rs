//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::collections::HashMap;
use std::hash::Hash;

use crate::ast::parser::Counter;
use crate::typings::Type;
use crate::{ast::lexer::Token, source::span::TextSpan};
use printer::Printer;
use termion::color::{Fg, Reset};
use visitor::Visitor;

pub mod evaluator;
pub mod lexer;
pub mod parser;
pub mod printer;
pub mod visitor;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct StmtId {
    pub id: usize,
}

impl StmtId {
    pub fn new(id: usize) -> Self {
        StmtId { id }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ExprId {
    pub id: usize,
}

impl ExprId {
    pub fn new(id: usize) -> Self {
        ExprId { id }
    }
}

#[derive(Debug, Clone)]
pub struct NodeIdGenerator {
    pub next_statement_id: Counter,
    pub next_expression_id: Counter,
}

impl NodeIdGenerator {
    pub fn new() -> Self {
        Self {
            next_statement_id: Counter::new(),
            next_expression_id: Counter::new(),
        }
    }

    pub fn next_statement_id(&self) -> StmtId {
        let id = self.next_statement_id.get_value();
        self.next_statement_id.increment();
        StmtId::new(id)
    }

    pub fn next_expression_id(&self) -> ExprId {
        let id = self.next_expression_id.get_value();
        self.next_expression_id.increment();
        ExprId::new(id)
    }
}

#[derive(Debug, Clone)]
pub struct Ast {
    pub statements: HashMap<StmtId, Statement>,
    pub expressions: HashMap<ExprId, Expression>,
    pub top_level_statements: Vec<StmtId>,
    pub node_id_generator: NodeIdGenerator,
}

impl Ast {
    pub fn new() -> Self {
        Self {
            statements: HashMap::new(),
            expressions: HashMap::new(),
            top_level_statements: Vec::new(),
            node_id_generator: NodeIdGenerator::new(),
        }
    }

    pub fn query_expr(&self, expr_id: ExprId) -> &Expression {
        &self.expressions[&expr_id]
    }

    pub fn query_stmt(&self, stmt_id: StmtId) -> &Statement {
        &self.statements[&stmt_id]
    }

    pub fn set_type(&mut self, expr_id: ExprId, expr_type: Type) {
        let expr = self.expressions.get_mut(&expr_id).unwrap();
        expr.expr_type = expr_type;
    }

    pub fn mark_top_level_statement(&mut self, stmt_id: StmtId) {
        self.top_level_statements.push(stmt_id);
    }

    fn stmt_from_kind(&mut self, kind: StatementKind) -> &Statement {
        let stmt = Statement::new(kind, self.node_id_generator.next_statement_id());
        let id = stmt.id;
        self.statements.insert(id, stmt);
        &self.statements[&id]
    }

    pub fn expression_statement(&mut self, expr_id: ExprId) -> &Statement {
        self.stmt_from_kind(StatementKind::Expression(expr_id))
    }

    pub fn let_statement(
        &mut self,
        identifier: Token,
        initializer: ExprId,
        type_annotation: Option<StaticTypeAnnotation>,
    ) -> &Statement {
        self.stmt_from_kind(StatementKind::Let(LetStatement {
            identifier,
            initializer,
            type_annotation,
        }))
    }

    pub fn if_statement(
        &mut self,
        if_keyword: Token,
        condition: ExprId,
        then: StmtId,
        else_statement: Option<ElseStatement>,
    ) -> &Statement {
        self.stmt_from_kind(StatementKind::If(IfStatement {
            if_keyword,
            condition,
            then_branch: then,
            else_branch: else_statement,
        }))
    }

    pub fn block_statement(&mut self, statements: Vec<StmtId>) -> &Statement {
        self.stmt_from_kind(StatementKind::Block(BlockStatement { statements }))
    }

    pub fn while_statement(
        &mut self,
        while_keyword: Token,
        condition: ExprId,
        body: StmtId,
    ) -> &Statement {
        self.stmt_from_kind(StatementKind::While(WhileStatement {
            while_keyword,
            condition,
            body,
        }))
    }

    pub fn return_statement(
        &mut self,
        return_keyword: Token,
        return_value: Option<ExprId>,
    ) -> &Statement {
        self.stmt_from_kind(StatementKind::Return(ReturnStatement {
            return_keyword,
            return_value,
        }))
    }

    pub fn func_decl_statement(
        &mut self,
        identifier: Token,
        parameters: Vec<FuncDeclParameter>,
        body: StmtId,
        return_type: Option<FunctionReturnType>,
    ) -> &Statement {
        self.stmt_from_kind(StatementKind::FuncDecl(FuncDeclStatement {
            identifier,
            parameters,
            body,
            return_type,
        }))
    }

    fn expr_from_kind(&mut self, kind: ExpressionKind) -> &Expression {
        let expr = Expression::new(
            kind,
            self.node_id_generator.next_expression_id(),
            Type::Unresolved,
        );
        let expr_id = expr.id;
        self.expressions.insert(expr_id, expr);
        &self.expressions[&expr_id]
    }

    pub fn number_expression(&mut self, token: Token, number: i64) -> &Expression {
        self.expr_from_kind(ExpressionKind::Number(NumberExpression { number, token }))
    }

    pub fn binary_expression(
        &mut self,
        operator: BinaryOperator,
        left: ExprId,
        right: ExprId,
    ) -> &Expression {
        self.expr_from_kind(ExpressionKind::Binary(BinaryExpression {
            left,
            operator,
            right,
        }))
    }

    pub fn parenthesized_expression(
        &mut self,
        left_paren: Token,
        expression: ExprId,
        right_paren: Token,
    ) -> &Expression {
        self.expr_from_kind(ExpressionKind::Parenthesized(ParenthesizedExpression {
            left_paren,
            expression,
            right_paren,
        }))
    }

    pub fn variable_expression(&mut self, identifier: Token) -> &Expression {
        self.expr_from_kind(ExpressionKind::Variable(VariableExpression { identifier }))
    }

    pub fn unary_expression(&mut self, operator: UnaryOperator, operand: ExprId) -> &Expression {
        self.expr_from_kind(ExpressionKind::Unary(UnaryExpression { operator, operand }))
    }

    pub fn assignment_expression(
        &mut self,
        identifier: Token,
        equals: Token,
        expression: ExprId,
    ) -> &Expression {
        self.expr_from_kind(ExpressionKind::Assignment(AssignmentExpression {
            identifier,
            equals,
            expression,
        }))
    }

    pub fn boolean_expression(&mut self, token: Token, value: bool) -> &Expression {
        self.expr_from_kind(ExpressionKind::Boolean(BooleanExpression { value, token }))
    }

    pub fn call_expression(
        &mut self,
        identifier: Token,
        left_paren: Token,
        arguments: Vec<ExprId>,
        right_paren: Token,
    ) -> &Expression {
        self.expr_from_kind(ExpressionKind::Call(CallExpression {
            identifier,
            left_paren,
            arguments,
            right_paren,
        }))
    }

    pub fn error_expression(&mut self, span: TextSpan) -> &Expression {
        self.expr_from_kind(ExpressionKind::Error(span))
    }

    pub fn visit(&self, visitor: &mut dyn Visitor) {
        for statement in &self.top_level_statements {
            visitor.visit_statement(*statement);
        }
    }

    pub fn visualize(&self) {
        let mut printer = Printer::new(self);
        self.visit(&mut printer);
        println!("{}", printer.result);
    }
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Expression(ExprId),
    Let(LetStatement),
    If(IfStatement),
    Block(BlockStatement),
    While(WhileStatement),
    FuncDecl(FuncDeclStatement),
    Return(ReturnStatement),
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub return_keyword: Token,
    pub return_value: Option<ExprId>,
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
pub struct FunctionReturnType {
    pub arrow: Token,
    pub type_name: Token,
}

impl FunctionReturnType {
    pub fn new(arrow: Token, type_name: Token) -> Self {
        Self { arrow, type_name }
    }
}

#[derive(Debug, Clone)]
pub struct FuncDeclParameter {
    pub identifier: Token,
    pub type_annotation: StaticTypeAnnotation,
}

#[derive(Debug, Clone)]
pub struct FuncDeclStatement {
    pub identifier: Token,
    pub parameters: Vec<FuncDeclParameter>,
    pub body: StmtId,
    pub return_type: Option<FunctionReturnType>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct WhileStatement {
    pub while_keyword: Token,
    pub condition: ExprId,
    pub body: StmtId,
}

#[derive(Debug, Clone)]
pub struct BlockStatement {
    pub statements: Vec<StmtId>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ElseStatement {
    pub else_keyword: Token,
    pub else_statement: StmtId,
}

impl ElseStatement {
    pub fn new(else_keyword: Token, else_statement: StmtId) -> Self {
        ElseStatement {
            else_keyword,
            else_statement,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IfStatement {
    pub if_keyword: Token,
    pub condition: ExprId,
    pub then_branch: StmtId,
    pub else_branch: Option<ElseStatement>,
}

#[derive(Debug, Clone)]
pub struct LetStatement {
    pub identifier: Token,
    pub initializer: ExprId,
    pub type_annotation: Option<StaticTypeAnnotation>,
}

#[derive(Debug, Clone)]
pub struct Statement {
    kind: StatementKind,
    id: StmtId,
}

impl Statement {
    pub fn new(kind: StatementKind, id: StmtId) -> Self {
        Statement { kind, id }
    }
}

#[derive(Debug, Clone)]
pub enum ExpressionKind {
    Number(NumberExpression),
    Binary(BinaryExpression),
    Unary(UnaryExpression),
    Parenthesized(ParenthesizedExpression),
    Variable(VariableExpression),
    Assignment(AssignmentExpression),
    Boolean(BooleanExpression),
    Call(CallExpression),
    Error(TextSpan),
}

#[derive(Debug, Clone)]
pub struct CallExpression {
    pub identifier: Token,
    pub left_paren: Token,
    pub arguments: Vec<ExprId>,
    pub right_paren: Token,
}

#[derive(Debug, Clone)]
pub struct BooleanExpression {
    pub value: bool,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct AssignmentExpression {
    pub identifier: Token,
    pub equals: Token,
    pub expression: ExprId,
}

#[derive(Debug, Clone)]
pub enum UnaryOperatorKind {
    Minus,
    BitwiseNot,
}

#[derive(Debug, Clone)]
pub struct UnaryOperator {
    pub(crate) kind: UnaryOperatorKind,
    token: Token,
}

impl UnaryOperator {
    pub fn new(kind: UnaryOperatorKind, token: Token) -> Self {
        Self { kind, token }
    }
}

#[derive(Debug, Clone)]
pub struct UnaryExpression {
    pub operator: UnaryOperator,
    pub operand: ExprId,
}

#[derive(Debug, Clone)]
pub struct VariableExpression {
    pub identifier: Token,
}

#[derive(Debug, Clone)]
pub enum BinaryOperatorKind {
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
pub enum BinaryOperatorAssociativity {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct BinaryOperator {
    pub kind: BinaryOperatorKind,
    pub token: Token,
}

impl BinaryOperator {
    pub fn new(kind: BinaryOperatorKind, token: Token) -> Self {
        BinaryOperator { kind, token }
    }

    pub fn precedence(&self) -> u8 {
        match self.kind {
            BinaryOperatorKind::Equals | BinaryOperatorKind::NotEquals => 30,
            BinaryOperatorKind::LessThan
            | BinaryOperatorKind::LessThanOrEqual
            | BinaryOperatorKind::GreaterThan
            | BinaryOperatorKind::GreaterThanOrEqual => 29,
            BinaryOperatorKind::Power => 20,
            BinaryOperatorKind::Multiply | BinaryOperatorKind::Divide => 19,
            BinaryOperatorKind::Plus | BinaryOperatorKind::Minus => 18,
            BinaryOperatorKind::LeftShift | BinaryOperatorKind::RightShift => 17,
            BinaryOperatorKind::BitwiseAnd => 16,
            BinaryOperatorKind::BitwiseXor => 15,
            BinaryOperatorKind::BitwiseOr => 14,
        }
    }

    pub fn associativity(&self) -> BinaryOperatorAssociativity {
        match self.kind {
            BinaryOperatorKind::Power => BinaryOperatorAssociativity::Right,
            _ => BinaryOperatorAssociativity::Left,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinaryExpression {
    pub left: ExprId,
    pub operator: BinaryOperator,
    pub right: ExprId,
}

#[derive(Debug, Clone)]
pub struct NumberExpression {
    pub number: i64,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct ParenthesizedExpression {
    pub left_paren: Token,
    pub expression: ExprId,
    pub right_paren: Token,
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub id: ExprId,
    pub expr_type: Type,
}

impl Expression {
    pub fn new(kind: ExpressionKind, id: ExprId, expr_type: Type) -> Self {
        Expression {
            kind,
            id,
            expr_type,
        }
    }

    pub fn span(&self, ast: &Ast) -> TextSpan {
        match &self.kind {
            ExpressionKind::Number(expr) => expr.token.span.clone(),
            ExpressionKind::Binary(expr) => {
                let left = ast.query_expr(expr.left).span(ast);
                let operator = expr.operator.token.span.clone();
                let right = ast.query_expr(expr.right).span(ast);
                TextSpan::combine(vec![left, operator, right])
            }
            ExpressionKind::Unary(expr) => {
                let operator = expr.operator.token.span.clone();
                let operand = ast.query_expr(expr.operand).span(ast);
                TextSpan::combine(vec![operator, operand])
            }
            ExpressionKind::Parenthesized(expr) => {
                let open_paren = expr.left_paren.span.clone();
                let expression = ast.query_expr(expr.expression).span(ast);
                let close_paren = expr.right_paren.span.clone();
                TextSpan::combine(vec![open_paren, expression, close_paren])
            }
            ExpressionKind::Variable(expr) => expr.identifier.span.clone(),
            ExpressionKind::Assignment(expr) => {
                let identifier = expr.identifier.span.clone();
                let equals = expr.equals.span.clone();
                let expression = ast.query_expr(expr.expression).span(ast);
                TextSpan::combine(vec![identifier, equals, expression])
            }
            ExpressionKind::Boolean(expr) => expr.token.span.clone(),
            ExpressionKind::Call(expr) => {
                let identifier = expr.identifier.span.clone();
                let left_paren = expr.left_paren.span.clone();
                let right_paren = expr.right_paren.span.clone();
                let mut spans = vec![identifier, left_paren, right_paren];
                for arg in &expr.arguments {
                    spans.push(ast.query_expr(*arg).span(ast));
                }
                TextSpan::combine(spans)
            }
            ExpressionKind::Error(span) => span.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ast::visitor::Visitor;
    use crate::compilation_unit::CompilationUnit;
    use crate::source::span::TextSpan;

    use super::{
        AssignmentExpression, Ast, BinaryExpression, BlockStatement, BooleanExpression,
        CallExpression, Expression, IfStatement, ReturnStatement, UnaryExpression, WhileStatement,
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

    struct Verifier {
        expected: Vec<TestAstNode>,
        actual: Vec<TestAstNode>,
        ast: Ast,
    }

    impl Verifier {
        pub fn new(input: &str, expected: Vec<TestAstNode>) -> Self {
            let compilation_unit = CompilationUnit::compile(input).expect("Failed to compile");
            let mut verifier = Verifier {
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

    impl Visitor for Verifier {
        fn get_ast(&self) -> &Ast {
            &self.ast
        }

        fn visit_func_decl_statement(&mut self, func_decl_statement: &super::FuncDeclStatement) {
            self.actual.push(TestAstNode::Func);
            self.visit_statement(func_decl_statement.body);
        }

        fn visit_return_statement(&mut self, return_statement: &ReturnStatement) {
            self.actual.push(TestAstNode::Return);
            if let Some(expr) = &return_statement.return_value {
                self.visit_expression(*expr);
            }
        }

        fn visit_let_statement(&mut self, let_statement: &super::LetStatement) {
            self.actual.push(TestAstNode::Let);
            self.visit_expression(let_statement.initializer);
        }

        fn visit_variable_expression(
            &mut self,
            variable_expression: &super::VariableExpression,
            _expr: &Expression,
        ) {
            self.actual.push(TestAstNode::Variable(
                variable_expression.identifier.span.literal.clone(),
            ));
        }

        fn visit_assignment_expression(
            &mut self,
            assignment_expression: &AssignmentExpression,
            _expr: &Expression,
        ) {
            self.actual.push(TestAstNode::Assignment);
            self.visit_expression(assignment_expression.expression);
        }

        fn visit_number_expression(
            &mut self,
            number: &super::NumberExpression,
            _expr: &Expression,
        ) {
            self.actual.push(TestAstNode::Number(number.number));
        }

        fn visit_error(&mut self, _span: &TextSpan) {
            // TODO
        }

        fn visit_unary_expression(
            &mut self,
            unary_expression: &UnaryExpression,
            _expr: &Expression,
        ) {
            self.actual.push(TestAstNode::Unary);
            self.visit_expression(unary_expression.operand);
        }

        fn visit_parenthesized_expression(
            &mut self,
            parenthesized_expression: &super::ParenthesizedExpression,
            _expr: &Expression,
        ) {
            self.actual.push(TestAstNode::Parenthesized);
            self.visit_expression(parenthesized_expression.expression);
        }

        fn visit_binary_expression(
            &mut self,
            binary_expression: &BinaryExpression,
            _expr: &Expression,
        ) {
            self.actual.push(TestAstNode::Binary);
            self.visit_expression(binary_expression.left);
            self.visit_expression(binary_expression.right);
        }

        fn visit_boolean_expression(&mut self, boolean: &BooleanExpression, _expr: &Expression) {
            self.actual.push(TestAstNode::Boolean(boolean.value));
        }

        fn visit_if_statement(&mut self, if_statement: &IfStatement) {
            self.actual.push(TestAstNode::If);
            self.visit_expression(if_statement.condition);
            self.visit_statement(if_statement.then_branch);
            if let Some(else_branch) = &if_statement.else_branch {
                self.actual.push(TestAstNode::Else);

                self.visit_statement(else_branch.else_statement);
            }
        }

        fn visit_while_statement(&mut self, while_statement: &WhileStatement) {
            self.actual.push(TestAstNode::While);
            self.visit_expression(while_statement.condition);
            self.visit_statement(while_statement.body);
        }

        fn visit_block_statement(&mut self, block_statement: &BlockStatement) {
            self.actual.push(TestAstNode::Block);
            for statement in &block_statement.statements {
                self.visit_statement(*statement);
            }
        }

        fn visit_call_expression(&mut self, call_expression: &CallExpression, _expr: &Expression) {
            self.actual.push(TestAstNode::Call);
            for argument in &call_expression.arguments {
                self.visit_expression(*argument);
            }
        }
    }

    fn assert_tree(input: &str, expected: Vec<TestAstNode>) {
        let verifier = Verifier::new(input, expected);
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
