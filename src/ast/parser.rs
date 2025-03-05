//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::cell::Cell;

use crate::diagnostics::DiagnosticsListCell;

use super::{
    AstBinaryOperator, AstBinaryOperatorKind, AstElseStatement, AstExpression, AstStatement,
    AstUnaryOperator, AstUnaryOperatorKind,
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
            TokenKind::If => self.parse_if_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_if_statement(&mut self) -> AstStatement {
        let if_keyword = self.consume_and_check(TokenKind::If).clone();
        let condition_expr = self.parse_expression();
        let then = self.parse_statement();
        let else_statement = self.parse_optional_else_statement();
        AstStatement::if_statement(if_keyword, condition_expr, then, else_statement)
    }

    fn parse_optional_else_statement(&mut self) -> Option<AstElseStatement> {
        if self.current().kind == TokenKind::Else {
            let else_keyword = self.consume_and_check(TokenKind::Else).clone();
            let else_statement = self.parse_statement();
            return Some(AstElseStatement::new(else_keyword, else_statement));
        }
        None
    }

    fn parse_let_statement(&mut self) -> AstStatement {
        self.consume_and_check(TokenKind::Let);
        let identifier = self.consume_and_check(TokenKind::Identifier).clone();
        self.consume_and_check(TokenKind::Equals);
        let expr = self.parse_expression();
        AstStatement::let_statement(identifier.clone(), expr)
    }

    fn parse_expression_statement(&mut self) -> AstStatement {
        let expr = self.parse_expression();
        AstStatement::expression(expr)
    }

    fn parse_expression(&mut self) -> AstExpression {
        self.parse_assignment_expression()
    }

    fn parse_assignment_expression(&mut self) -> AstExpression {
        if self.current().kind == TokenKind::Identifier && self.peek(1).kind == TokenKind::Equals {
            let identifier = self.consume_and_check(TokenKind::Identifier).clone();
            self.consume_and_check(TokenKind::Equals);
            let expr = self.parse_expression();
            return AstExpression::assignment(identifier, expr);
        }
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(&mut self, precedence: u8) -> AstExpression {
        let mut left = self.parse_unary_expression();

        while let Some(operator) = self.parse_binary_operator() {
            let operator_precedence = operator.precedence();
            if operator_precedence < precedence {
                break;
            }
            self.consume();
            let right = self.parse_binary_expression(operator_precedence);
            left = AstExpression::binary(operator, left, right);
        }

        left
    }

    fn parse_unary_expression(&mut self) -> AstExpression {
        if let Some(operator) = self.parse_unary_operator() {
            self.consume();
            let operand = self.parse_unary_expression();
            return AstExpression::unary(operator, operand);
        }
        self.parse_primary_expression()
    }

    fn parse_unary_operator(&mut self) -> Option<AstUnaryOperator> {
        let token = self.current();
        let kind = match token.kind {
            TokenKind::Minus => Some(AstUnaryOperatorKind::Minus),
            TokenKind::Tilde => Some(AstUnaryOperatorKind::BitwiseNot),
            _ => None,
        };
        kind.map(|kind| AstUnaryOperator::new(kind, token.clone()))
    }

    fn parse_binary_operator(&mut self) -> Option<AstBinaryOperator> {
        let token = self.current();
        let kind = match token.kind {
            TokenKind::Plus => Some(AstBinaryOperatorKind::Plus),
            TokenKind::Minus => Some(AstBinaryOperatorKind::Minus),
            TokenKind::Asterisk => Some(AstBinaryOperatorKind::Multiply),
            TokenKind::Slash => Some(AstBinaryOperatorKind::Divide),
            TokenKind::Ampersand => Some(AstBinaryOperatorKind::BitwiseAnd),
            TokenKind::Pipe => Some(AstBinaryOperatorKind::BitwiseOr),
            TokenKind::Caret => Some(AstBinaryOperatorKind::BitwiseXor),
            TokenKind::DoubleAsterisk => Some(AstBinaryOperatorKind::Power),
            TokenKind::DoubleLessThan => Some(AstBinaryOperatorKind::LeftShift),
            TokenKind::DoubleGreaterThan => Some(AstBinaryOperatorKind::RightShift),
            TokenKind::EqualsEquals => Some(AstBinaryOperatorKind::Equals),
            TokenKind::BangEquals => Some(AstBinaryOperatorKind::NotEquals),
            TokenKind::LessThan => Some(AstBinaryOperatorKind::LessThan),
            TokenKind::LessThanEquals => Some(AstBinaryOperatorKind::LessThanOrEqual),
            TokenKind::GreaterThan => Some(AstBinaryOperatorKind::GreaterThan),
            TokenKind::GreaterThanEquals => Some(AstBinaryOperatorKind::GreaterThanOrEqual),
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
