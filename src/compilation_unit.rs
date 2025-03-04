use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::AstVisitor;
use crate::ast::evaluator::AstEvaluator;
use crate::ast::lexer::Lexer;
use crate::ast::parser::Parser;
use crate::{ast, diagnostics, source};
use crate::{ast::Ast, diagnostics::DiagnosticsListCell};

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
        if !self
            .symbols
            .contains_key(&variable_expression.identifier.span.literal)
        {
            let mut diagnostics_binding = self.diagnostics_list.borrow_mut();
            diagnostics_binding.report_undeclared_variable(&variable_expression.identifier);
        }
    }

    fn visit_number_expression(&mut self, number: &ast::AstNumberExpression) {}

    fn visit_error(&mut self, span: &ast::lexer::TextSpan) {}

    fn visit_unary_expression(&mut self, unary_expression: &ast::AstUnaryExpression) {
        self.visit_expression(&unary_expression.operand);
    }
}

pub struct CompilationUnit {
    pub ast: Ast,
    pub diagnostics_list: DiagnosticsListCell,
}

impl CompilationUnit {
    pub fn compile(input: &str) -> CompilationUnit {
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

        if Self::check_diagnostics(&text, &diagnostics_list).is_err() {
            return Self::create_compilation_unit(ast, diagnostics_list);
        }
        let mut symbol_checker = SymbolChecker::new(Rc::clone(&diagnostics_list));
        ast.visit(&mut symbol_checker);
        if Self::check_diagnostics(&text, &diagnostics_list).is_err() {
            return Self::create_compilation_unit(ast, diagnostics_list);
        }

        Self::create_compilation_unit(ast, diagnostics_list)
    }

    pub fn maybe_run(&self) {
        if self.diagnostics_list.borrow().diagnostics.len() > 0 {
            return;
        }
        self.run();
    }

    fn run(&self) {
        let mut eval = AstEvaluator::new();
        self.ast.visit(&mut eval);
        println!("Result: {:?}", eval.last_value);
    }

    fn create_compilation_unit(ast: Ast, diagnostics_list: DiagnosticsListCell) -> CompilationUnit {
        CompilationUnit {
            ast,
            diagnostics_list,
        }
    }

    fn check_diagnostics(
        text: &source::SourceText,
        diagnostics_list: &DiagnosticsListCell,
    ) -> Result<(), ()> {
        let diagnostics_binding = diagnostics_list.borrow();
        if !diagnostics_binding.diagnostics.is_empty() {
            let diagnostics_printer = diagnostics::printer::DiagnosticsPrinter::new(
                text,
                &diagnostics_binding.diagnostics,
            );
            diagnostics_printer.print();
            return Err(());
        }
        Ok(())
    }
}
