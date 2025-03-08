//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use crate::source::span::TextSpan;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenKind {
    // Literals
    Number(i64),
    // Operators
    Plus,
    Minus,
    Asterisk,
    Slash,
    Equals,
    Ampersand,
    Pipe,
    Caret,
    DoubleAsterisk,
    Tilde,
    DoubleLessThan,
    DoubleGreaterThan,
    EqualsEquals,
    BangEquals,
    LessThan,
    LessThanEquals,
    GreaterThan,
    GreaterThanEquals,
    // Keywords
    Let,
    If,
    Else,
    True,
    False,
    While,
    Func,
    Return,
    // Separators
    LeftParen,
    RightParen,
    OpenBrace,
    CloseBrace,
    Comma,
    Colon,
    Arrow,
    // Other
    Semicolon,
    Whitespace,
    Identifier,
    Eof,
    Invalid,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Number(_) => write!(f, "Number"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Asterisk => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::Eof => write!(f, "EOF"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Whitespace => write!(f, "Whitespace"),
            TokenKind::Invalid => write!(f, "Invalid"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Identifier => write!(f, "Identifier"),
            TokenKind::Equals => write!(f, "="),
            TokenKind::Ampersand => write!(f, "&"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::DoubleAsterisk => write!(f, "**"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::DoubleLessThan => write!(f, "<<"),
            TokenKind::DoubleGreaterThan => write!(f, ">>"),
            TokenKind::If => write!(f, "If"),
            TokenKind::Else => write!(f, "Else"),
            TokenKind::GreaterThan => write!(f, ">"),
            TokenKind::LessThan => write!(f, "<"),
            TokenKind::GreaterThanEquals => write!(f, ">="),
            TokenKind::LessThanEquals => write!(f, "<="),
            TokenKind::EqualsEquals => write!(f, "=="),
            TokenKind::BangEquals => write!(f, "!="),
            TokenKind::OpenBrace => write!(f, "{{"),
            TokenKind::CloseBrace => write!(f, "}}"),
            TokenKind::True => write!(f, "True"),
            TokenKind::False => write!(f, "False"),
            TokenKind::While => write!(f, "While"),
            TokenKind::Func => write!(f, "Func"),
            TokenKind::Return => write!(f, "Return"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Arrow => write!(f, "->"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: TextSpan,
}

impl Token {
    pub fn new(kind: TokenKind, span: TextSpan) -> Self {
        Self { kind, span }
    }
}

pub struct Lexer<'a> {
    input: &'a str,
    current_pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            current_pos: 0,
        }
    }

    pub fn next_token(&mut self) -> Option<Token> {
        if self.current_pos == self.input.len() {
            self.current_pos += 1;
            return Some(Token::new(
                TokenKind::Eof,
                TextSpan::new(0, 0, '\0'.to_string()),
            ));
        }
        let c = self.current_char();
        c.map(|c| {
            let start: usize = self.current_pos;
            let kind;
            if Self::is_number_start(c) {
                let number: i64 = self.consume_number();
                kind = TokenKind::Number(number);
            } else if Self::is_whitespace(c) {
                self.consume();
                kind = TokenKind::Whitespace;
            } else if Self::is_identifier_start(c) {
                let identifier = self.consume_identifier();
                kind = match identifier.as_str() {
                    "let" => TokenKind::Let,
                    "if" => TokenKind::If,
                    "else" => TokenKind::Else,
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    "while" => TokenKind::While,
                    "func" => TokenKind::Func,
                    "return" => TokenKind::Return,
                    _ => TokenKind::Identifier,
                }
            } else {
                kind = self.consume_punctuation();
            }
            let end: usize = self.current_pos;
            let literal: String = self.input[start..end].to_string();
            let span: TextSpan = TextSpan::new(start, end, literal);
            Token::new(kind, span)
        })
    }

    fn consume_punctuation(&mut self) -> TokenKind {
        let c = self.consume().unwrap();
        match c {
            '+' => TokenKind::Plus,
            '-' => self.lex_potential_double_char_operator('>', TokenKind::Minus, TokenKind::Arrow),
            '*' => self.lex_potential_double_char_operator(
                '*',
                TokenKind::Asterisk,
                TokenKind::DoubleAsterisk,
            ),
            '/' => TokenKind::Slash,
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '=' => self.lex_potential_double_char_operator(
                '=',
                TokenKind::Equals,
                TokenKind::EqualsEquals,
            ),
            ';' => TokenKind::Semicolon,
            '&' => TokenKind::Ampersand,
            '|' => TokenKind::Pipe,
            '^' => TokenKind::Caret,
            '~' => TokenKind::Tilde,
            '<' => {
                let mut kind_map = HashMap::new();
                kind_map.insert('=', TokenKind::LessThanEquals);
                kind_map.insert('<', TokenKind::DoubleLessThan);
                self.lex_potential_double_char_operator_multiple_kinds(
                    TokenKind::LessThan,
                    &kind_map,
                )
            }
            '>' => {
                let mut kind_map = HashMap::new();
                kind_map.insert('=', TokenKind::GreaterThanEquals);
                kind_map.insert('>', TokenKind::DoubleGreaterThan);
                self.lex_potential_double_char_operator_multiple_kinds(
                    TokenKind::GreaterThan,
                    &kind_map,
                )
            }
            '!' => self.lex_potential_double_char_operator(
                '=',
                TokenKind::Invalid,
                TokenKind::BangEquals,
            ),
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            _ => TokenKind::Invalid,
        }
    }

    fn lex_potential_double_char_operator_multiple_kinds(
        &mut self,
        one_char_kind: TokenKind,
        double_char_kinds: &HashMap<char, TokenKind>,
    ) -> TokenKind {
        if let Some(next) = self.current_char() {
            let mut kind = &one_char_kind;
            for (expected, double_kind) in double_char_kinds {
                if next == *expected {
                    self.consume();
                    kind = double_kind;
                    break;
                }
            }
            *kind
        } else {
            one_char_kind
        }
    }

    fn lex_potential_double_char_operator(
        &mut self,
        expected: char,
        one_char_kind: TokenKind,
        double_char_kind: TokenKind,
    ) -> TokenKind {
        if let Some(next) = self.current_char() {
            if next == expected {
                self.consume();
                double_char_kind
            } else {
                one_char_kind
            }
        } else {
            one_char_kind
        }
    }

    fn is_number_start(c: char) -> bool {
        c.is_ascii_digit()
    }

    fn is_identifier_start(c: char) -> bool {
        c.is_alphabetic()
    }

    fn is_whitespace(c: char) -> bool {
        c.is_whitespace()
    }

    fn current_char(&self) -> Option<char> {
        self.input.chars().nth(self.current_pos)
    }

    fn consume(&mut self) -> Option<char> {
        if self.current_pos >= self.input.len() {
            return None;
        }
        let c = self.current_char();
        self.current_pos += 1;

        c
    }

    fn consume_identifier(&mut self) -> String {
        let mut identifier = String::new();
        while let Some(c) = self.current_char() {
            if Self::is_identifier_start(c) {
                self.consume().unwrap();
                identifier.push(c);
            } else {
                break;
            }
        }
        identifier
    }

    fn consume_number(&mut self) -> i64 {
        let mut number: i64 = 0;
        while let Some(c) = self.current_char() {
            if c.is_ascii_digit() {
                self.consume().unwrap();
                number = number * 10 + i64::from(c.to_digit(10).unwrap());
            } else {
                break;
            }
        }
        number
    }
}
