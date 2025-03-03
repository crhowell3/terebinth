//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::cell::Cell;

use crate::diagnostics::DiagnosticsListCell;

use super::{
    Ast, AstBinaryOperator, AstBinaryOperatorKind, AstExpression, AstStatement,
    lexer::{Token, TokenKind},
};

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

pub struct Parser {
    tokens: Vec<Token>,
    current: Counter,
    diagnostics_list: DiagnosticsListCell,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, diagnostics_list: DiagnosticsListCell) -> Self {
        Self {
            tokens: tokens
                .iter()
                .filter(|token| token.kind != TokenKind::Whitespace)
                .cloned()
                .collect(),
            current: Counter::new(),
            diagnostics_list,
        }
    }

    pub fn next_statement(&mut self) -> Option<AstStatement> {
        if self.is_at_end() {
            return None;
        }
        Some(self.parse_statement())
    }

    fn is_at_end(&self) -> bool {
        self.current().kind == TokenKind::Eof
    }

    fn parse_statement(&mut self) -> AstStatement {
        match self.current().kind {
            TokenKind::Let => self.parse_let_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_expression_statement(&mut self) -> AstStatement {
        let expr = self.parse_expression();
        AstStatement::expression(expr)
    }

    fn parse_let_statement(&mut self) -> AstStatement {
        self.consume_and_check(TokenKind::Let);
        let identifier = self.consume_and_check(TokenKind::Identifier).clone();
        self.consume_and_check(TokenKind::Equals);
        let expr = self.parse_expression();
        AstStatement::let_statement(identifier.clone(), expr)
    }

    fn parse_expression(&mut self) -> AstExpression {
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(&mut self, precedence: u8) -> AstExpression {
        let mut left = self.parse_primary_expression();

        while let Some(operator) = self.parse_binary_operator() {
            self.consume();
            let operator_precedence = operator.precedence();
            if operator_precedence < precedence {
                break;
            }
            let right = self.parse_binary_expression(operator_precedence);
            left = AstExpression::binary(operator, left, right);
        }

        left
    }

    fn parse_binary_operator(&mut self) -> Option<AstBinaryOperator> {
        let token = self.current();
        let kind = match token.kind {
            TokenKind::Plus => Some(AstBinaryOperatorKind::Plus),
            TokenKind::Minus => Some(AstBinaryOperatorKind::Minus),
            TokenKind::Asterisk => Some(AstBinaryOperatorKind::Multiply),
            TokenKind::Slash => Some(AstBinaryOperatorKind::Divide),
            _ => None,
        };
        kind.map(|kind| AstBinaryOperator::new(kind, token.clone()))
    }

    fn parse_primary_expression(&mut self) -> AstExpression {
        let token = self.consume();
        match token.kind {
            TokenKind::Number(number) => AstExpression::number(number),
            TokenKind::LeftParen => {
                let expr = self.parse_expression();
                self.consume_and_check(TokenKind::RightParen);
                AstExpression::parenthesized(expr)
            }
            TokenKind::Identifier => AstExpression::identifier(token.clone()),
            _ => {
                self.diagnostics_list
                    .borrow_mut()
                    .report_expected_expression(token);
                AstExpression::error(token.span.clone())
            }
        }
    }

    fn peek(&self, offset: isize) -> &Token {
        let mut index = (self.current.get_value() as isize + offset) as usize;
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
