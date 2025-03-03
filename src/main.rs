//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ast::AstVisitor;
use ast::evaluator::AstEvaluator;
use diagnostics::DiagnosticsListCell;
use source::SourceText;

use crate::ast::Ast;
use crate::ast::lexer::Lexer;
use crate::ast::parser::Parser;

mod ast;
mod diagnostics;
mod source;

struct SymbolChecker {
    symbols: HashMap<String, ()>,
    diagnostics_list: DiagnosticsListCell,
}

impl SymbolChecker {
    fn new(diagnostics_list: DiagnosticsListCell) -> Self {
        Self {
            symbols: HashMap::new(),
            diagnostics_list,
        }
    }
}

impl AstVisitor for SymbolChecker {
    fn visit_let_statement(&mut self, let_statement: &ast::AstLetStatement) {
        let identifier = let_statement.identifier.span.literal.clone();
        self.visit_expression(&let_statement.initializer);
        self.symbols.insert(identifier, ());
    }

    fn visit_variable_expression(&mut self, variable_expression: &ast::AstVariableExpression) {
        if self
            .symbols
            .get(&variable_expression.identifier.span.literal)
            .is_none()
        {
            let mut diagnostics_binding = self.diagnostics_list.borrow_mut();
            diagnostics_binding.report_undeclared_variable(&variable_expression.identifier);
        }
    }

    fn visit_number_expression(&mut self, number: &ast::AstNumberExpression) {}

    fn visit_error(&mut self, span: &ast::lexer::TextSpan) {}
}

fn main() -> Result<(), ()> {
    let input = "
        let a =10+30
        let b = 20
        let d = 10 + e
        let c = (a + b) * d
    ";
    let text = source::SourceText::new(input.to_string());

    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token() {
        tokens.push(token);
    }
    let diagnostics_list: DiagnosticsListCell =
        Rc::new(RefCell::new(diagnostics::DiagnosticsList::new()));
    let mut ast: Ast = Ast::new();
    let mut parser = Parser::new(tokens, Rc::clone(&diagnostics_list));
    while let Some(statement) = parser.next_statement() {
        ast.add_statement(statement);
    }
    ast.visualize();

    check_diagnostics(&text, &diagnostics_list)?;
    let mut symbol_checker = SymbolChecker::new(Rc::clone(&diagnostics_list));
    ast.visit(&mut symbol_checker);
    check_diagnostics(&text, &diagnostics_list)?;
    let mut eval = AstEvaluator::new();
    ast.visit(&mut eval);
    println!("Result: {:?}", eval.last_value);
    Ok(())
}

fn check_diagnostics(text: &SourceText, diagnostics_list: &DiagnosticsListCell) -> Result<(), ()> {
    let diagnostics_binding = diagnostics_list.borrow();
    if !diagnostics_binding.diagnostics.is_empty() {
        let diagnostics_printer =
            diagnostics::printer::DiagnosticsPrinter::new(text, &diagnostics_binding.diagnostics);
        diagnostics_printer.print();
        return Err(());
    }
    Ok(())
}
