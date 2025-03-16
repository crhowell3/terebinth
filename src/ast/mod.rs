//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::hash::Hash;

use crate::compilation_unit::VariableIndex;
use crate::typings::Type;
use crate::{ast::lexer::Token, source::span::TextSpan};
use printer::Printer;
use terebinth::{Idx, IdxVec, idx};
use termion::color::{Fg, Reset};
use visitor::Visitor;

pub mod evaluator;
pub mod lexer;
pub mod parser;
pub mod printer;
pub mod visitor;

idx!(StmtId);
idx!(ExprId);
idx!(ItemId);

#[cfg_attr(test, derive(Clone))]
#[derive(Debug)]
pub struct Ast {
    pub statements: IdxVec<StmtId, Stmt>,
    pub expressions: IdxVec<ExprId, Expr>,
    pub items: IdxVec<ItemId, Item>,
}

impl Ast {
    pub fn new() -> Self {
        Self {
            statements: IdxVec::new(),
            expressions: IdxVec::new(),
            items: IdxVec::new(),
        }
    }

    pub fn query_item(&self, item_id: ItemId) -> &Item {
        &self.items[item_id]
    }

    pub fn query_expr(&self, expr_id: ExprId) -> &Expr {
        &self.expressions[expr_id]
    }

    fn query_expr_mut(&mut self, expr_id: ExprId) -> &mut Expr {
        &mut self.expressions[expr_id]
    }

    pub fn query_stmt(&self, stmt_id: StmtId) -> &Stmt {
        &self.statements[stmt_id]
    }

    fn query_stmt_mut(&mut self, stmt_id: StmtId) -> &mut Stmt {
        &mut self.statements[stmt_id]
    }

    pub fn set_variable(&mut self, expr_id: ExprId, variable_idx: VariableIndex) {
        let expr = self.query_expr_mut(expr_id);
        match &mut expr.kind {
            ExprKind::Assignment(assign_expr) => {
                assign_expr.variable_idx = variable_idx;
            }
            ExprKind::Variable(var_expr) => {
                var_expr.variable_idx = variable_idx;
            }
            _ => unreachable!("Cannot set variable of non-variable expression"),
        }
    }

    pub fn set_variable_for_stmt(&mut self, stmt_id: StmtId, variable_idx: VariableIndex) {
        let stmt = self.query_stmt_mut(stmt_id);
        match &mut stmt.kind {
            StmtKind::Let(var_decl) => {
                var_decl.variable_idx = variable_idx;
            }
            _ => unreachable!("Cannot set variable of non-variable statement"),
        }
    }

    pub fn set_type(&mut self, expr_id: ExprId, expr_type: Type) {
        let expr = &mut self.expressions[expr_id];
        expr.ty = expr_type;
    }

    pub fn stmt_from_kind(&mut self, kind: StmtKind) -> &Stmt {
        let stmt = Stmt::new(kind, StmtId::new(0));
        let id = self.statements.push(stmt);
        self.statements[id].id = id;
        &self.statements[id]
    }

    pub fn expression_statement(&mut self, expr_id: ExprId) -> &Stmt {
        self.stmt_from_kind(StmtKind::Expr(expr_id))
    }

    pub fn let_statement(
        &mut self,
        identifier: Token,
        initializer: ExprId,
        type_annotation: Option<StaticTypeAnnotation>,
    ) -> &Stmt {
        self.stmt_from_kind(StmtKind::Let(LetStmt {
            identifier,
            initializer,
            type_annotation,
            variable_idx: VariableIndex::new(0),
        }))
    }

    pub fn if_expr(
        &mut self,
        if_keyword: Token,
        condition: ExprId,
        then: ExprId,
        else_statement: Option<ElseBranch>,
    ) -> &Expr {
        self.expr_from_kind(ExprKind::If(IfExpr {
            if_keyword,
            condition,
            then_branch: then,
            else_branch: else_statement,
        }))
    }

    pub fn while_statement(
        &mut self,
        while_keyword: Token,
        condition: ExprId,
        body: ExprId,
    ) -> &Stmt {
        self.stmt_from_kind(StmtKind::While(WhileStmt {
            while_keyword,
            condition,
            body,
        }))
    }

    pub fn block_expression(
        &mut self,
        left_brace: Token,
        stmts: Vec<StmtId>,
        right_brace: Token,
    ) -> &Expr {
        self.expr_from_kind(ExprKind::Block(BlockExpr {
            left_brace,
            stmts,
            right_brace,
        }))
    }

    pub fn return_statement(
        &mut self,
        return_keyword: Token,
        return_value: Option<ExprId>,
    ) -> &Stmt {
        self.stmt_from_kind(StmtKind::Return(ReturnStmt {
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
    ) -> &Item {
        self.item_from_kind(ItemKind::Func(FunctionDeclaration {
            identifier,
            parameters,
            body,
            return_type,
        }))
    }

    pub fn item_from_kind(&mut self, kind: ItemKind) -> &Item {
        let item = Item::new(kind, ItemId::new(0));
        let id = self.items.push(item);
        self.items[id].id = id;
        &self.items[id]
    }

    fn expr_from_kind(&mut self, kind: ExprKind) -> &Expr {
        let expr = Expr::new(kind, ExprId::new(0), Type::Unresolved);
        let id = self.expressions.push(expr);
        self.expressions[id].id = id;
        &self.expressions[id]
    }

    pub fn number_expression(&mut self, token: Token, number: i64) -> &Expr {
        self.expr_from_kind(ExprKind::Number(NumberExpr { number, token }))
    }

    pub fn binary_expression(
        &mut self,
        operator: BinaryOperator,
        left: ExprId,
        right: ExprId,
    ) -> &Expr {
        self.expr_from_kind(ExprKind::Binary(BinaryExpr {
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
    ) -> &Expr {
        self.expr_from_kind(ExprKind::Parenthesized(ParenthesizedExpr {
            left_paren,
            expression,
            right_paren,
        }))
    }

    pub fn variable_expression(&mut self, identifier: Token) -> &Expr {
        self.expr_from_kind(ExprKind::Variable(VariableExpr {
            identifier,
            variable_idx: VariableIndex::new(0),
        }))
    }

    pub fn unary_expression(&mut self, operator: UnaryOperator, operand: ExprId) -> &Expr {
        self.expr_from_kind(ExprKind::Unary(UnaryExpr { operator, operand }))
    }

    pub fn assignment_expression(
        &mut self,
        identifier: Token,
        equals: Token,
        expression: ExprId,
    ) -> &Expr {
        self.expr_from_kind(ExprKind::Assignment(AssignmentExpr {
            identifier,
            equals,
            expression,
            variable_idx: VariableIndex::new(0),
        }))
    }

    pub fn boolean_expression(&mut self, token: Token, value: bool) -> &Expr {
        self.expr_from_kind(ExprKind::Boolean(BooleanExpr { value, token }))
    }

    pub fn call_expression(
        &mut self,
        identifier: Token,
        left_paren: Token,
        arguments: Vec<ExprId>,
        right_paren: Token,
    ) -> &Expr {
        self.expr_from_kind(ExprKind::Call(CallExpr {
            identifier,
            left_paren,
            arguments,
            right_paren,
        }))
    }

    pub fn error_expression(&mut self, span: TextSpan) -> &Expr {
        self.expr_from_kind(ExprKind::Error(span))
    }

    pub fn visit(&mut self, visitor: &mut dyn Visitor) {
        for item in self.items.clone().iter() {
            visitor.visit_item(self, item.id);
        }
    }

    pub fn visualize(&mut self) {
        let mut printer = Printer::new();
        self.visit(&mut printer);
        println!("{}", printer.result);
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: ItemId,
    pub kind: ItemKind,
}

impl Item {
    pub fn new(kind: ItemKind, id: ItemId) -> Self {
        Self { id, kind }
    }
}

#[derive(Debug, Clone)]
pub enum ItemKind {
    Func(FunctionDeclaration),
    Stmt(StmtId),
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Expr(ExprId),
    Let(LetStmt),
    While(WhileStmt),
    Return(ReturnStmt),
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
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
pub struct FunctionDeclaration {
    pub identifier: Token,
    pub parameters: Vec<FuncDeclParameter>,
    pub body: StmtId,
    pub return_type: Option<FunctionReturnType>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub while_keyword: Token,
    pub condition: ExprId,
    pub body: ExprId,
}

#[derive(Debug, Clone)]
pub struct BlockExpr {
    pub left_brace: Token,
    pub stmts: Vec<StmtId>,
    pub right_brace: Token,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ElseBranch {
    pub else_keyword: Token,
    pub expr: ExprId,
}

impl ElseBranch {
    pub fn new(else_keyword: Token, expr: ExprId) -> Self {
        ElseBranch { else_keyword, expr }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IfExpr {
    pub if_keyword: Token,
    pub condition: ExprId,
    pub then_branch: ExprId,
    pub else_branch: Option<ElseBranch>,
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub identifier: Token,
    pub initializer: ExprId,
    pub type_annotation: Option<StaticTypeAnnotation>,
    pub variable_idx: VariableIndex,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub id: StmtId,
}

impl Stmt {
    pub fn new(kind: StmtKind, id: StmtId) -> Self {
        Stmt { kind, id }
    }

    pub fn span(&self, ast: &Ast) -> TextSpan {
        match &self.kind {
            StmtKind::Expr(expr_id) => ast.query_expr(*expr_id).span(ast),
            StmtKind::Let(let_stmt) => {
                let mut spans = vec![let_stmt.identifier.span.clone()];
                if let Some(type_annotation) = &let_stmt.type_annotation {
                    spans.push(type_annotation.colon.span.clone());
                    spans.push(type_annotation.type_name.span.clone());
                }
                TextSpan::combine(spans)
            }
            StmtKind::While(while_stmt) => {
                let mut spans = vec![while_stmt.while_keyword.span.clone()];
                spans.push(ast.query_expr(while_stmt.condition).span(ast));
                spans.push(ast.query_expr(while_stmt.body).span(ast));
                TextSpan::combine(spans)
            }
            StmtKind::Return(return_stmt) => {
                let mut spans = vec![return_stmt.return_keyword.span.clone()];
                if let Some(return_value) = &return_stmt.return_value {
                    spans.push(ast.query_expr(*return_value).span(ast));
                }
                TextSpan::combine(spans)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Number(NumberExpr),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Parenthesized(ParenthesizedExpr),
    Variable(VariableExpr),
    Assignment(AssignmentExpr),
    Boolean(BooleanExpr),
    Call(CallExpr),
    If(IfExpr),
    Block(BlockExpr),
    Error(TextSpan),
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub identifier: Token,
    pub left_paren: Token,
    pub arguments: Vec<ExprId>,
    pub right_paren: Token,
}

#[derive(Debug, Clone)]
pub struct BooleanExpr {
    pub value: bool,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct AssignmentExpr {
    pub identifier: Token,
    pub equals: Token,
    pub expression: ExprId,
    pub variable_idx: VariableIndex,
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
pub struct UnaryExpr {
    pub operator: UnaryOperator,
    pub operand: ExprId,
}

#[derive(Debug, Clone)]
pub struct VariableExpr {
    pub identifier: Token,
    pub variable_idx: VariableIndex,
}

#[derive(Debug, Clone)]
pub enum BinaryOperatorKind {
    // Arithmetic
    Plus,
    Minus,
    Multiply,
    Divide,
    Power,
    // Bitwise
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
    // Relational
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
pub struct BinaryExpr {
    pub left: ExprId,
    pub operator: BinaryOperator,
    pub right: ExprId,
}

#[derive(Debug, Clone)]
pub struct NumberExpr {
    pub number: i64,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct ParenthesizedExpr {
    pub left_paren: Token,
    pub expression: ExprId,
    pub right_paren: Token,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub id: ExprId,
    pub ty: Type,
}

impl Expr {
    pub fn new(kind: ExprKind, id: ExprId, expr_type: Type) -> Self {
        Expr {
            kind,
            id,
            ty: expr_type,
        }
    }

    pub fn span(&self, ast: &Ast) -> TextSpan {
        match &self.kind {
            ExprKind::Number(expr) => expr.token.span.clone(),
            ExprKind::Binary(expr) => {
                let left = ast.query_expr(expr.left).span(ast);
                let operator = expr.operator.token.span.clone();
                let right = ast.query_expr(expr.right).span(ast);
                TextSpan::combine(vec![left, operator, right])
            }
            ExprKind::Unary(expr) => {
                let operator = expr.operator.token.span.clone();
                let operand = ast.query_expr(expr.operand).span(ast);
                TextSpan::combine(vec![operator, operand])
            }
            ExprKind::Parenthesized(expr) => {
                let open_paren = expr.left_paren.span.clone();
                let expression = ast.query_expr(expr.expression).span(ast);
                let close_paren = expr.right_paren.span.clone();
                TextSpan::combine(vec![open_paren, expression, close_paren])
            }
            ExprKind::Variable(expr) => expr.identifier.span.clone(),
            ExprKind::Assignment(expr) => {
                let identifier = expr.identifier.span.clone();
                let equals = expr.equals.span.clone();
                let expression = ast.query_expr(expr.expression).span(ast);
                TextSpan::combine(vec![identifier, equals, expression])
            }
            ExprKind::Boolean(expr) => expr.token.span.clone(),
            ExprKind::Call(expr) => {
                let identifier = expr.identifier.span.clone();
                let left_paren = expr.left_paren.span.clone();
                let right_paren = expr.right_paren.span.clone();
                let mut spans = vec![identifier, left_paren, right_paren];
                for arg in &expr.arguments {
                    spans.push(ast.query_expr(*arg).span(ast));
                }
                TextSpan::combine(spans)
            }
            ExprKind::If(expr) => {
                let if_span = expr.if_keyword.span.clone();
                let condition = ast.query_expr(expr.condition).span(ast);
                let then_branch = ast.query_expr(expr.then_branch).span(ast);
                let mut spans = vec![if_span, condition, then_branch];
                if let Some(else_branch) = &expr.else_branch {
                    let else_span = else_branch.else_keyword.span.clone();
                    spans.push(else_span);
                    spans.push(ast.query_expr(else_branch.expr).span(ast));
                }
                TextSpan::combine(spans)
            }
            ExprKind::Block(expr) => {
                let mut spans = vec![expr.left_brace.span.clone()];
                for statement in &expr.stmts {
                    spans.push(ast.query_stmt(*statement).span(ast));
                }
                spans.push(expr.right_brace.span.clone());
                TextSpan::combine(spans)
            }
            ExprKind::Error(span) => span.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ast::visitor::Visitor;
    use crate::compilation_unit::CompilationUnit;
    use crate::source::span::TextSpan;

    use super::{
        AssignmentExpr, Ast, BinaryExpr, BlockExpr, BooleanExpr, CallExpr, Expr, IfExpr, LetStmt,
        ReturnStmt, Stmt, UnaryExpr, WhileStmt,
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
            let mut ast = self.ast.clone();
            ast.visit(&mut *self);
        }

        pub fn verify(&self) {
            assert_eq!(
                self.expected.len(),
                self.actual.len(),
                "Expected {} nodes, but got {}. Actual nodes: {:?}",
                self.expected.len(),
                self.actual.len(),
                self.actual
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
        fn visit_func_decl_stmt(
            &mut self,
            ast: &mut Ast,
            func_decl_statement: &super::FunctionDeclaration,
        ) {
            self.actual.push(TestAstNode::Func);
            self.visit_stmt(ast, func_decl_statement.body);
        }

        fn visit_return_stmt(&mut self, ast: &mut Ast, return_statement: &ReturnStmt) {
            self.actual.push(TestAstNode::Return);
            if let Some(expr) = &return_statement.return_value {
                self.visit_expr(ast, *expr);
            }
        }

        fn visit_let_stmt(&mut self, ast: &mut Ast, let_statement: &LetStmt, _stmt: &Stmt) {
            self.actual.push(TestAstNode::Let);
            self.visit_expr(ast, let_statement.initializer);
        }

        fn visit_variable_expr(
            &mut self,
            _ast: &mut Ast,
            variable_expression: &super::VariableExpr,
            _expr: &Expr,
        ) {
            self.actual.push(TestAstNode::Variable(
                variable_expression.identifier.span.literal.clone(),
            ));
        }

        fn visit_assignment_expr(
            &mut self,
            ast: &mut Ast,
            assignment_expression: &AssignmentExpr,
            _expr: &Expr,
        ) {
            self.actual.push(TestAstNode::Assignment);
            self.visit_expr(ast, assignment_expression.expression);
        }

        fn visit_number_expr(&mut self, _ast: &mut Ast, number: &super::NumberExpr, _expr: &Expr) {
            self.actual.push(TestAstNode::Number(number.number));
        }

        fn visit_error(&mut self, _ast: &mut Ast, _span: &TextSpan) {
            // TODO
        }

        fn visit_unary_expr(&mut self, ast: &mut Ast, unary_expression: &UnaryExpr, _expr: &Expr) {
            self.actual.push(TestAstNode::Unary);
            self.visit_expr(ast, unary_expression.operand);
        }

        fn visit_parenthesized_expr(
            &mut self,
            ast: &mut Ast,
            parenthesized_expression: &super::ParenthesizedExpr,
            _expr: &Expr,
        ) {
            self.actual.push(TestAstNode::Parenthesized);
            self.visit_expr(ast, parenthesized_expression.expression);
        }

        fn visit_binary_expr(
            &mut self,
            ast: &mut Ast,
            binary_expression: &BinaryExpr,
            _expr: &Expr,
        ) {
            self.actual.push(TestAstNode::Binary);
            self.visit_expr(ast, binary_expression.left);
            self.visit_expr(ast, binary_expression.right);
        }

        fn visit_boolean_expr(&mut self, _ast: &mut Ast, boolean: &BooleanExpr, _expr: &Expr) {
            self.actual.push(TestAstNode::Boolean(boolean.value));
        }

        fn visit_if_expr(&mut self, ast: &mut Ast, if_statement: &IfExpr, _expr: &Expr) {
            self.actual.push(TestAstNode::If);
            self.visit_expr(ast, if_statement.condition);
            self.visit_expr(ast, if_statement.then_branch);
            if let Some(else_branch) = &if_statement.else_branch {
                self.actual.push(TestAstNode::Else);

                self.visit_expr(ast, else_branch.expr);
            }
        }

        fn visit_while_stmt(&mut self, ast: &mut Ast, while_statement: &WhileStmt) {
            self.actual.push(TestAstNode::While);
            self.visit_expr(ast, while_statement.condition);
            self.visit_expr(ast, while_statement.body);
        }

        fn visit_block_expr(&mut self, ast: &mut Ast, block_statement: &BlockExpr, _expr: &Expr) {
            self.actual.push(TestAstNode::Block);
            for statement in &block_statement.stmts {
                self.visit_stmt(ast, *statement);
            }
        }

        fn visit_call_expr(&mut self, ast: &mut Ast, call_expression: &CallExpr, _expr: &Expr) {
            self.actual.push(TestAstNode::Call);
            for argument in &call_expression.arguments {
                self.visit_expr(ast, *argument);
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
