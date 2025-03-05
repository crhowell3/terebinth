use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::evaluator::AstEvaluator;
use crate::ast::lexer::Lexer;
use crate::ast::parser::Parser;
use crate::ast::{AstBlockStatement, visitor::AstVisitor};
use crate::{ast, diagnostics, source};
use crate::{ast::Ast, diagnostics::DiagnosticsListCell};

struct Scope {
    symbols: HashMap<String, ()>,
}

impl Scope {
    fn new() -> Self {
        Scope {
            symbols: HashMap::new(),
        }
    }

    fn declare(&mut self, identifier: &str) {
        self.symbols.insert(identifier.to_string(), ());
    }

    fn lookup(&self, identifier: &str) -> bool {
        self.symbols.contains_key(identifier)
    }
}

struct Scopes {
    scopes: Vec<Scope>,
}

impl Scopes {
    fn new() -> Self {
        Scopes { scopes: Vec::new() }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, identifier: &str) {
        self.scopes.last_mut().unwrap().declare(identifier);
    }

    fn lookup(&self, identifier: &str) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|scope| scope.lookup(identifier))
    }
}

struct Resolver {
    scopes: Scopes,
    diagnostics_list: DiagnosticsListCell,
}

impl Resolver {
    fn new(diagnostics_list: DiagnosticsListCell) -> Self {
        let mut scopes = Scopes::new();
        scopes.enter_scope();
        Self {
            scopes,
            diagnostics_list,
        }
    }
}

impl AstVisitor<'_> for Resolver {
    fn visit_block_statement(&mut self, block_statement: &AstBlockStatement) {
        self.scopes.enter_scope();
        for statement in &block_statement.statements {
            self.visit_statement(statement);
        }
        self.scopes.exit_scope();
    }
    fn visit_let_statement(&mut self, let_statement: &ast::AstLetStatement) {
        let identifier = let_statement.identifier.span.literal.clone();
        self.visit_expression(&let_statement.initializer);
        self.scopes.declare(&identifier);
    }

    fn visit_variable_expression(&mut self, variable_expression: &ast::AstVariableExpression) {
        if !self
            .scopes
            .lookup(&variable_expression.identifier.span.literal)
        {
            let mut diagnostics_binding = self.diagnostics_list.borrow_mut();
            diagnostics_binding.report_undeclared_variable(&variable_expression.identifier);
        }
    }

    fn visit_number_expression(&mut self, _number: &ast::AstNumberExpression) {}

    fn visit_error(&mut self, _span: &ast::lexer::TextSpan) {}

    fn visit_unary_expression(&mut self, unary_expression: &ast::AstUnaryExpression) {
        self.visit_expression(&unary_expression.operand);
    }

    fn visit_boolean_expression(&mut self, boolean: &ast::AstBooleanExpression) {}
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
        let mut resolver = Resolver::new(Rc::clone(&diagnostics_list));
        ast.visit(&mut resolver);
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
