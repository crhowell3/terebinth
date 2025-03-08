//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::{cell::RefCell, rc::Rc};

use crate::ast::lexer::{Token, TokenKind};
use crate::source::span::TextSpan;
use crate::typings::Type;

pub mod printer;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum DiagnosticKind {
    Error,
    Warning,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
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

#[derive(Debug)]
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

    #[allow(dead_code)]
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

    pub fn report_undeclared_function(&mut self, token: &Token) {
        self.report_error(
            format!("Undeclared function '{}'", token.span.literal),
            token.span.clone(),
        );
    }

    pub fn report_invalid_argument_count(&mut self, token: &Token, expected: usize, actual: usize) {
        self.report_error(
            format!(
                "Function '{}' expects {} arguments, but was given {}",
                token.span.literal, expected, actual
            ),
            token.span.clone(),
        );
    }

    pub fn report_function_already_declared(&mut self, token: &Token) {
        self.report_error(
            format!("Function '{}' already declared", token.span.literal),
            token.span.clone(),
        );
    }

    pub fn report_type_mismatch(&mut self, span: &TextSpan, expected: &Type, actual: &Type) {
        self.report_error(
            format!("Expected type '{}', found '{}'", expected, actual),
            span.clone(),
        );
    }

    pub fn report_undeclared_type(&mut self, token: &Token) {
        self.report_error(
            format!("Undeclared type '{}'", token.span.literal),
            token.span.clone(),
        );
    }

    pub fn report_cannot_return_outside_function(&mut self, token: &Token) {
        self.report_error(
            "Cannot use 'return' outside of function scope".to_string(),
            token.span.clone(),
        );
    }
}

#[cfg(test)]
mod test {
    use crate::compilation_unit::CompilationUnit;
    use crate::diagnostics::{Diagnostic, DiagnosticKind};
    use crate::source::span::TextSpan;

    struct DiagnosticsVerifier {
        actual: Vec<Diagnostic>,
        expected: Vec<Diagnostic>,
    }

    impl DiagnosticsVerifier {
        pub fn new(input: &str, messages: Vec<&str>) -> Self {
            let messages_len = messages.len();
            let expected = Self::parse_input(input, messages);
            assert_eq!(expected.len(), messages_len);
            let actual = Self::compile(input);
            Self { expected, actual }
        }

        fn compile(input: &str) -> Vec<Diagnostic> {
            let raw = Self::get_raw_text(input);
            let compilation_unit = CompilationUnit::compile(&raw);
            match compilation_unit {
                Ok(_) => vec![],
                Err(e) => e.borrow().diagnostics.clone(),
            }
        }

        fn get_raw_text(input: &str) -> String {
            input.replace("«", "").replace("»", "")
        }

        fn parse_input(input: &str, messages: Vec<&str>) -> Vec<Diagnostic> {
            let raw = Self::get_raw_text(input);
            let mut start_index_stack = vec![];
            let mut current_position: usize = 0;
            let mut diagnostics = vec![];
            for c in input.chars() {
                match c {
                    '«' => {
                        start_index_stack.push(current_position);
                    }
                    '»' => {
                        let start_index = start_index_stack.pop().unwrap();
                        let end_index = current_position;
                        let literal = &raw[start_index..end_index];
                        let span = TextSpan::new(start_index, end_index, literal.to_string());
                        let message = messages[diagnostics.len()].to_string();
                        let diagnostic = Diagnostic::new(message, span, DiagnosticKind::Error);
                        diagnostics.push(diagnostic);
                    }
                    _ => {
                        current_position += 1;
                    }
                };
            }

            diagnostics
        }

        fn verify(&self) {
            assert_eq!(
                self.actual.len(),
                self.expected.len(),
                "Expected {} diagnostics, found {}",
                self.expected.len(),
                self.actual.len()
            );

            for (actual, expected) in self.actual.iter().zip(self.expected.iter()) {
                assert_eq!(
                    actual.message, expected.message,
                    "Expected message '{}', found '{}'",
                    expected.message, actual.message
                );
                assert_eq!(
                    actual.span.start, expected.span.start,
                    "Expected start index {}, found {}",
                    expected.span.start, actual.span.start
                );
                assert_eq!(
                    actual.span.end, expected.span.end,
                    "Expected end index {}, found {}",
                    expected.span.end, actual.span.end
                );
                assert_eq!(
                    actual.span.literal, expected.span.literal,
                    "Expected literal '{}', found '{}'",
                    expected.span.literal, actual.span.literal
                );
            }
        }
    }

    fn assert_diagnostics(input: &str, expected: Vec<&str>) {
        let verifier = DiagnosticsVerifier::new(input, expected);
        verifier.verify();
    }

    #[test]
    fn should_report_undeclared_variable() {
        let input = "let a = «b»";
        let expected = vec!["Undeclared variable 'b'"];

        assert_diagnostics(input, expected);
    }

    #[test]
    fn should_report_expected_expression() {
        let input = "let a = «+»";
        let expected = vec!["Expected expression, found <+>"];

        assert_diagnostics(input, expected);
    }

    #[test]
    fn should_report_invalid_token() {
        let input = "let a = 8 «@» 2";
        let expected = vec!["Expected expression, found <Invalid>"];

        assert_diagnostics(input, expected);
    }

    #[test]
    fn should_report_undeclared_variable_if_declared_in_if_without_block() {
        let input = "\
        let b = -1
        if b > 10
            let a = 10
        «a»
        ";
        let expected = vec!["Undeclared variable 'a'"];

        assert_diagnostics(input, expected);
    }

    #[test]
    fn should_report_function_already_declared() {
        let input = "\
        func a() {}
        func «a»() {}
        ";

        let expected = vec!["Function 'a' already declared"];

        assert_diagnostics(input, expected);
    }
}
