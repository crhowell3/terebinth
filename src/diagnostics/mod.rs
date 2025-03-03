//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::{cell::RefCell, rc::Rc};

use crate::ast::lexer::{TextSpan, Token, TokenKind};

pub mod printer;

pub enum DiagnosticKind {
    Error,
    Warning,
}

pub struct Diagnostic {
    pub message: String,
    pub span: TextSpan,
    pub kind: DiagnosticKind,
}

impl Diagnostic {
    pub fn new(message: String, span: TextSpan, kind: DiagnosticKind) -> Self {
        Diagnostic {
            message,
            span,
            kind,
        }
    }
}

pub type DiagnosticsListCell = Rc<RefCell<DiagnosticsList>>;

pub struct DiagnosticsList {
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticsList {
    pub fn new() -> Self {
        DiagnosticsList {
            diagnostics: vec![],
        }
    }

    pub fn report_error(&mut self, message: String, span: TextSpan) {
        let error = Diagnostic::new(message, span, DiagnosticKind::Error);
        self.diagnostics.push(error);
    }

    pub fn report_warning(&mut self, message: String, span: TextSpan) {
        let warning = Diagnostic::new(message, span, DiagnosticKind::Warning);
        self.diagnostics.push(warning);
    }

    pub fn report_unexpected_token(&mut self, expected: &TokenKind, token: &Token) {
        self.report_error(
            format!("Expected <{}>, found <{}>", expected, token.kind),
            token.span.clone(),
        );
    }

    pub fn report_expected_expression(&mut self, token: &Token) {
        self.report_error(
            format!("Expected expression, found <{}>", token.kind),
            token.span.clone(),
        );
    }

    pub fn report_undeclared_variable(&mut self, token: &Token) {
        self.report_error(
            format!("Undeclared variable '{}'", token.span.literal),
            token.span.clone(),
        );
    }
}
