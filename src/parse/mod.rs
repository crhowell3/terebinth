//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::cell::Cell;

use crate::diagnostics::DiagnosticsListCell;
use crate::lexer::{Token, TokenKind};

use crate::ast::{
    Ast, BinaryOperator, BinaryOperatorAssociativity, BinaryOperatorKind, ElseBranch, Expr, ExprId,
    FuncDeclParameter, FunctionReturnType, Item, ItemKind, StaticTypeAnnotation, Stmt, StmtId,
    UnaryOperator, UnaryOperatorKind,
};

mod lexer;

#[derive(Debug, Clone)]
pub struct Counter {
    value: Cell<usize>,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            value: Cell::new(0),
        }
    }

    pub fn increment(&self) {
        let current_value = self.value.get();
        self.value.set(current_value + 1);
    }

    pub fn get_value(&self) -> usize {
        self.value.get()
    }
}

pub struct Parser<'a> {
    tokens: Vec<Token>,
    current: Counter,
    diagnostics_list: DiagnosticsListCell,
    ast: &'a mut Ast,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &[Token], diagnostics_list: DiagnosticsListCell, ast: &'a mut Ast) -> Self {
        Self {
            tokens: tokens
                .iter()
                .filter(|token| token.kind != TokenKind::Whitespace)
                .cloned()
                .collect(),
            current: Counter::new(),
            diagnostics_list,
            ast,
        }
    }

    pub fn parse(&mut self) {
        while self.next_item().map(|stmt| stmt.id).is_some() {}
    }

    fn next_item(&mut self) -> Option<&Item> {
        if self.is_at_end() {
            return None;
        }
        Some(self.parse_item())
    }

    fn is_at_end(&self) -> bool {
        self.current().kind == TokenKind::Eof
    }

    fn parse_item(&mut self) -> &Item {
        if self.current().kind == TokenKind::Func {
            self.parse_function_declaration()
        } else {
            let id = self.parse_statement();
            self.ast.item_from_kind(ItemKind::Stmt(id))
        }
    }

    fn parse_statement(&mut self) -> StmtId {
        let stmt = match self.current().kind {
            TokenKind::Let => self.parse_let_statement().id,
            TokenKind::While => self.parse_while_statement().id,
            TokenKind::Return => self.parse_return_statement().id,
            _ => self.parse_expression_statement().id,
        };
        self.consume_if(TokenKind::Semi);
        stmt
    }

    fn parse_function_declaration(&mut self) -> &Item {
        self.consume_and_check(TokenKind::Func);
        let identifier = self.consume_and_check(TokenKind::Ident).clone();
        let parameters = self.parse_optional_parameter_list();
        let return_type = self.parse_optional_return_type();
        let body = self.parse_statement();
        self.ast
            .func_decl_statement(identifier, parameters, body, return_type)
    }

    fn parse_optional_return_type(&mut self) -> Option<FunctionReturnType> {
        if self.current().kind == TokenKind::Arrow {
            let arrow = self.consume_and_check(TokenKind::Arrow).clone();
            let type_name = self.consume_and_check(TokenKind::Ident).clone();
            return Some(FunctionReturnType::new(arrow, type_name));
        }
        None
    }

    fn parse_optional_parameter_list(&mut self) -> Vec<FuncDeclParameter> {
        if self.current().kind != TokenKind::OpenParen {
            return Vec::new();
        }
        self.consume_and_check(TokenKind::OpenParen);
        let mut parameters = Vec::new();
        while self.current().kind != TokenKind::CloseParen && !self.is_at_end() {
            parameters.push(FuncDeclParameter {
                identifier: self.consume_and_check(TokenKind::Ident).clone(),
                type_annotation: self.parse_type_annotation(),
            });
            if self.current().kind == TokenKind::Comma {
                self.consume_and_check(TokenKind::Comma);
            }
        }
        self.consume_and_check(TokenKind::CloseParen);
        parameters
    }

    fn parse_return_statement(&mut self) -> &Stmt {
        let return_keyword = self.consume_and_check(TokenKind::Return).clone();
        let expression = self.parse_expression().id;
        self.ast.return_statement(return_keyword, Some(expression))
    }

    fn parse_while_statement(&mut self) -> &Stmt {
        let while_keyword = self.consume_and_check(TokenKind::While).clone();
        let condition_expr = self.parse_expression().id;
        let body = self.parse_expression().id;
        self.ast
            .while_statement(while_keyword, condition_expr, body)
    }

    fn parse_block_expr(&mut self, left_brace: Token) -> &Expr {
        let mut statements = Vec::new();
        while self.current().kind != TokenKind::CloseBrace && !self.is_at_end() {
            statements.push(self.parse_statement());
        }
        let right_brace = self.consume_and_check(TokenKind::CloseBrace).clone();
        self.ast
            .block_expression(left_brace, statements, right_brace)
    }

    fn parse_if_expression(&mut self, if_keyword: Token) -> &Expr {
        let condition_expr = self.parse_expression().id;
        let then = self.parse_expression().id;
        let else_statement = self.parse_optional_else_statement();
        self.ast
            .if_expr(if_keyword, condition_expr, then, else_statement)
    }

    fn parse_optional_else_statement(&mut self) -> Option<ElseBranch> {
        if self.current().kind == TokenKind::Else {
            let else_keyword = self.consume_and_check(TokenKind::Else).clone();
            let else_expr = self.parse_expression().id;
            return Some(ElseBranch::new(else_keyword, else_expr));
        }
        None
    }

    fn parse_let_statement(&mut self) -> &Stmt {
        self.consume_and_check(TokenKind::Let);
        let identifier = self.consume_and_check(TokenKind::Ident).clone();
        let optional_type_annotation = self.parse_optional_type_annotation();
        self.consume_and_check(TokenKind::Eq);
        let expr = self.parse_expression().id;
        self.ast
            .let_statement(identifier.clone(), expr, optional_type_annotation)
    }

    fn parse_optional_type_annotation(&mut self) -> Option<StaticTypeAnnotation> {
        if self.current().kind == TokenKind::Colon {
            return Some(self.parse_type_annotation());
        }
        None
    }

    fn parse_type_annotation(&mut self) -> StaticTypeAnnotation {
        let colon = self.consume_and_check(TokenKind::Colon).clone();
        let type_name = self.consume_and_check(TokenKind::Ident).clone();
        StaticTypeAnnotation::new(colon, type_name)
    }

    fn parse_expression_statement(&mut self) -> &Stmt {
        let expr = self.parse_expression().id;
        self.ast.expression_statement(expr)
    }

    fn parse_expression(&mut self) -> &Expr {
        self.parse_assignment_expression()
    }

    fn parse_assignment_expression(&mut self) -> &Expr {
        if self.current().kind == TokenKind::Ident && self.peek(1).kind == TokenKind::Eq {
            let identifier = self.consume_and_check(TokenKind::Ident).clone();
            let equals = self.consume_and_check(TokenKind::Eq).clone();
            let expr = self.parse_expression().id;
            return self.ast.assignment_expression(identifier, equals, expr);
        }
        self.parse_binary_expression()
    }

    fn parse_binary_expression(&mut self) -> &Expr {
        let left = self.parse_unary_expression().id;
        self.parse_binary_expression_recurse(left, 0)
    }

    fn parse_binary_expression_recurse(&mut self, mut left: ExprId, precedence: u8) -> &Expr {
        while let Some(operator) = self.parse_binary_operator() {
            let operator_precedence = operator.precedence();
            if operator_precedence < precedence {
                break;
            }
            self.consume();
            let mut right = self.parse_unary_expression().id;

            while let Some(inner_operator) = self.parse_binary_operator() {
                let greater_precedence = inner_operator.precedence() > operator.precedence();
                let equal_precedence = inner_operator.precedence() == operator.precedence();
                if !(greater_precedence
                    || equal_precedence
                        && inner_operator.associativity() == BinaryOperatorAssociativity::Right)
                {
                    break;
                }

                right = self
                    .parse_binary_expression_recurse(
                        right,
                        std::cmp::max(operator.precedence(), inner_operator.precedence()),
                    )
                    .id;
            }
            left = self.ast.binary_expression(operator, left, right).id;
        }

        self.ast.query_expr(left)
    }

    fn parse_unary_expression(&mut self) -> &Expr {
        if let Some(operator) = self.parse_unary_operator() {
            self.consume();
            let operand = self.parse_unary_expression().id;
            return self.ast.unary_expression(operator, operand);
        }
        self.parse_primary_expression()
    }

    fn parse_unary_operator(&mut self) -> Option<UnaryOperator> {
        let token = self.current();
        let kind = match token.kind {
            TokenKind::Minus => Some(UnaryOperatorKind::Minus),
            TokenKind::Tilde => Some(UnaryOperatorKind::BitwiseNot),
            _ => None,
        };
        kind.map(|kind| UnaryOperator::new(kind, token.clone()))
    }

    fn parse_binary_operator(&mut self) -> Option<BinaryOperator> {
        let token = self.current();
        let kind = match token.kind {
            TokenKind::Plus => Some(BinaryOperatorKind::Plus),
            TokenKind::Minus => Some(BinaryOperatorKind::Minus),
            TokenKind::Star => Some(BinaryOperatorKind::Multiply),
            TokenKind::Slash => Some(BinaryOperatorKind::Divide),
            TokenKind::Ampersand => Some(BinaryOperatorKind::BitwiseAnd),
            TokenKind::Pipe => Some(BinaryOperatorKind::BitwiseOr),
            TokenKind::Caret => Some(BinaryOperatorKind::BitwiseXor),
            TokenKind::DoubleAsterisk => Some(BinaryOperatorKind::Power),
            TokenKind::DoubleLessThan => Some(BinaryOperatorKind::LeftShift),
            TokenKind::DoubleGreaterThan => Some(BinaryOperatorKind::RightShift),
            TokenKind::EqualsEquals => Some(BinaryOperatorKind::Equals),
            TokenKind::BangEquals => Some(BinaryOperatorKind::NotEquals),
            TokenKind::Lt => Some(BinaryOperatorKind::LessThan),
            TokenKind::LessThanEquals => Some(BinaryOperatorKind::LessThanOrEqual),
            TokenKind::Gt => Some(BinaryOperatorKind::GreaterThan),
            TokenKind::GreaterThanEquals => Some(BinaryOperatorKind::GreaterThanOrEqual),
            _ => None,
        };
        kind.map(|kind| BinaryOperator::new(kind, token.clone()))
    }

    fn parse_primary_expression(&mut self) -> &Expr {
        let token = self.consume().clone();
        match token.kind {
            TokenKind::OpenBrace => self.parse_block_expr(token),
            TokenKind::If => self.parse_if_expression(token),
            TokenKind::Number(number) => self.ast.number_expression(token, number),
            TokenKind::OpenParen => {
                let expr = self.parse_expression().id;
                let left_paren = token;
                let right_paren = self.consume_and_check(TokenKind::CloseParen).clone();
                self.ast
                    .parenthesized_expression(left_paren, expr, right_paren)
            }
            TokenKind::Ident => {
                if self.current().kind == TokenKind::OpenParen {
                    self.parse_call_expression(&token.clone())
                } else {
                    self.ast.variable_expression(token)
                }
            }
            TokenKind::True | TokenKind::False => {
                let value = token.kind == TokenKind::True;
                self.ast.boolean_expression(token, value)
            }
            _ => {
                self.diagnostics_list
                    .borrow_mut()
                    .report_expected_expression(&token);
                self.ast.error_expression(token.span.clone())
            }
        }
    }

    fn parse_call_expression(&mut self, identifier: &Token) -> &Expr {
        let left_paren = self.consume_and_check(TokenKind::OpenParen).clone();
        let mut arguments = Vec::new();
        while self.current().kind != TokenKind::CloseParen && !self.is_at_end() {
            arguments.push(self.parse_expression().id);
            if self.current().kind != TokenKind::CloseParen {
                self.consume_and_check(TokenKind::Comma);
            }
        }
        let right_paren = self.consume_and_check(TokenKind::CloseParen).clone();
        self.ast
            .call_expression(identifier.clone(), left_paren, arguments, right_paren)
    }

    fn peek(&self, offset: isize) -> &Token {
        let mut index =
            usize::try_from(isize::try_from(self.current.get_value()).ok().unwrap() + offset)
                .ok()
                .unwrap();
        if index >= self.tokens.len() {
            index = self.tokens.len() - 1;
        }
        self.tokens.get(index).unwrap()
    }

    fn current(&self) -> &Token {
        self.peek(0)
    }

    fn consume(&self) -> &Token {
        self.current.increment();
        self.peek(-1)
    }

    fn consume_if(&self, kind: TokenKind) -> Option<&Token> {
        if self.current().kind == kind {
            Some(self.consume())
        } else {
            None
        }
    }

    fn consume_and_check(&self, kind: TokenKind) -> &Token {
        let token = self.consume();
        if token.kind != kind {
            self.diagnostics_list
                .borrow_mut()
                .report_unexpected_token(&kind, token);
        }
        token
    }
}
