//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::cell::RefCell;
use std::rc::Rc;

use ast::evaluator::AstEvaluator;
use diagnostics::DiagnosticsListCell;

use crate::ast::Ast;
use crate::ast::lexer::Lexer;
use crate::ast::parser::Parser;

mod ast;
mod diagnostics;
mod source;

fn main() {
    let input = "7 + 8 * 9";
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

    let diagnostics_binding = diagnostics_list.borrow();
    if !diagnostics_binding.diagnostics.is_empty() {
        let diagnostics_printer =
            diagnostics::printer::DiagnosticsPrinter::new(&text, &diagnostics_binding.diagnostics);
        diagnostics_printer.print();
        return;
    }

    let mut eval = AstEvaluator::new();
    ast.visit(&mut eval);
    println!("Result: {:?}", eval.last_value);
}
