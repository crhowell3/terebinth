use termion::color;

use crate::source::span::TextSpan;

use crate::ast::{
    AssignmentExpr, Ast, BinaryExpr, BlockExpr, BooleanExpr, CallExpr, Expr, Fg,
    FunctionDeclaration, IfExpr, LetStmt, NumberExpr, ParenthesizedExpr, Reset, ReturnStmt,
    StaticTypeAnnotation, StmtId, UnaryExpr, VariableExpr, Visitor, WhileStmt,
};

use super::Stmt;

pub struct Printer {
    indent: usize,
    pub result: String,
}

impl Printer {
    const NUMBER_COLOR: color::Magenta = color::Magenta;
    const TEXT_COLOR: color::LightWhite = color::LightWhite;
    const KEYWORD_COLOR: color::Blue = color::Blue;
    const VARIABLE_COLOR: color::Green = color::Green;
    const BOOLEAN_COLOR: color::LightMagenta = color::LightMagenta;
    const TYPE_COLOR: color::LightYellow = color::LightYellow;

    fn add_whitespace(&mut self) {
        self.result.push(' ');
    }

    fn add_newline(&mut self) {
        self.result.push('\n');
    }

    fn add_keyword(&mut self, keyword: &str) {
        self.result
            .push_str(&format!("{}{}", Self::KEYWORD_COLOR.fg_str(), keyword));
    }

    fn add_text(&mut self, text: &str) {
        self.result
            .push_str(&format!("{}{}", Self::TEXT_COLOR.fg_str(), text));
    }

    fn add_variable(&mut self, variable: &str) {
        self.result
            .push_str(&format!("{}{}", Self::VARIABLE_COLOR.fg_str(), variable));
    }

    fn add_indent(&mut self) {
        for _ in 0..self.indent {
            self.result.push_str("  ");
        }
    }

    fn add_boolean_literal(&mut self, boolean: bool) {
        self.result
            .push_str(&format!("{}{}", Self::BOOLEAN_COLOR.fg_str(), boolean));
    }

    fn add_type(&mut self, type_: &str) {
        self.result
            .push_str(&format!("{}{}", Self::TYPE_COLOR.fg_str(), type_,));
    }

    fn add_type_annotation(&mut self, type_annotation: &StaticTypeAnnotation) {
        self.add_text(":");
        self.add_whitespace();
        self.add_type(&type_annotation.type_name.span.literal);
    }

    pub fn new() -> Self {
        Self {
            indent: 0,
            result: String::new(),
        }
    }
}

impl Visitor for Printer {
    fn visit_func_decl_statement(
        &mut self,
        ast: &mut Ast,
        func_decl_statement: &FunctionDeclaration,
    ) {
        self.add_keyword("func");
        self.add_whitespace();
        self.add_text(&func_decl_statement.identifier.span.literal);
        self.add_text("(");
        for (i, parameter) in func_decl_statement.parameters.iter().enumerate() {
            if i != 0 {
                self.add_text(",");
                self.add_whitespace();
            }
            self.add_text(&parameter.identifier.span.literal);
            self.add_type_annotation(&parameter.type_annotation);
        }
        self.add_text(")");
        self.add_whitespace();
        if let Some(return_type) = &func_decl_statement.return_type {
            self.add_text("->");
            self.add_whitespace();
            self.add_type(&return_type.type_name.span.literal);
            self.add_whitespace();
        }
        self.visit_stmt(ast, func_decl_statement.body);
    }

    fn visit_call_expression(&mut self, ast: &mut Ast, call_expression: &CallExpr, _expr: &Expr) {
        self.add_text(&call_expression.identifier.span.literal);
        self.add_text("(");
        for (i, argument) in call_expression.arguments.iter().enumerate() {
            if i != 0 {
                self.add_text(",");
                self.add_whitespace();
            }
            self.visit_expr(ast, *argument);
        }
        self.add_text(")");
    }

    fn visit_return_statement(&mut self, ast: &mut Ast, return_statement: &ReturnStmt) {
        self.add_keyword("return");
        if let Some(expr) = &return_statement.return_value {
            self.add_whitespace();
            self.visit_expr(ast, *expr);
        }
    }

    fn visit_boolean_expression(&mut self, _ast: &mut Ast, boolean: &BooleanExpr, _expr: &Expr) {
        self.add_boolean_literal(boolean.value);
    }

    fn visit_while_statement(&mut self, ast: &mut Ast, while_statement: &WhileStmt) {
        self.add_keyword("while");
        self.add_whitespace();
        self.visit_expr(ast, while_statement.condition);
        self.add_whitespace();
        self.visit_expr(ast, while_statement.body);
    }

    fn visit_if_expr(&mut self, ast: &mut Ast, if_statement: &IfExpr, _expr: &Expr) {
        self.add_keyword("if");
        self.add_whitespace();
        self.visit_expr(ast, if_statement.condition);
        self.add_whitespace();
        self.visit_expr(ast, if_statement.then_branch);
        if let Some(else_branch) = &if_statement.else_branch {
            self.add_keyword("else");
            self.add_whitespace();
            self.visit_expr(ast, else_branch.expr);
        }
    }

    fn visit_let_statement(&mut self, ast: &mut Ast, let_statement: &LetStmt, _stmt: &Stmt) {
        self.add_keyword("let");
        self.add_whitespace();
        self.add_text(let_statement.identifier.span.literal.as_str());
        if let Some(type_annotation) = &let_statement.type_annotation {
            self.add_type_annotation(type_annotation);
        }
        self.add_whitespace();
        self.add_text("=");
        self.add_whitespace();
        self.visit_expr(ast, let_statement.initializer);
    }

    fn visit_assignment_expression(
        &mut self,
        ast: &mut Ast,
        assignment_expression: &AssignmentExpr,
        _expr: &Expr,
    ) {
        self.add_variable(assignment_expression.identifier.span.literal.as_str());
        self.add_whitespace();
        self.add_text("=");
        self.add_whitespace();
        self.visit_expr(ast, assignment_expression.expression);
    }

    fn visit_stmt(&mut self, ast: &mut Ast, statement: StmtId) {
        self.add_indent();
        self.do_visit_statement(ast, statement);
        self.result.push_str(&format!("{}\n", Fg(Reset)));
    }

    fn visit_block_expr(&mut self, ast: &mut Ast, block_statement: &BlockExpr, _expr: &Expr) {
        self.add_text("{");
        self.add_newline();
        self.indent += 1;
        for statement in &block_statement.stmts {
            self.visit_stmt(ast, *statement);
        }
        self.indent -= 1;
        self.add_indent();
        self.add_text("}");
    }

    fn visit_number_expression(&mut self, _ast: &mut Ast, number: &NumberExpr, _expr: &Expr) {
        self.result
            .push_str(&format!("{}{}", Self::NUMBER_COLOR.fg_str(), number.number));
    }

    fn visit_error(&mut self, _ast: &mut Ast, span: &TextSpan) {
        self.result
            .push_str(&format!("{}{}", Self::TEXT_COLOR.fg_str(), span.literal));
    }

    fn visit_unary_expression(
        &mut self,
        ast: &mut Ast,
        unary_expression: &UnaryExpr,
        _expr: &Expr,
    ) {
        self.result.push_str(&format!(
            "{}{}",
            Self::TEXT_COLOR.fg_str(),
            unary_expression.operator.token.span.literal
        ));
        self.visit_expr(ast, unary_expression.operand);
    }

    fn visit_binary_expression(
        &mut self,
        ast: &mut Ast,
        binary_expression: &BinaryExpr,
        _expr: &Expr,
    ) {
        self.visit_expr(ast, binary_expression.left);
        self.add_whitespace();
        self.result.push_str(&format!(
            "{}{}",
            Self::TEXT_COLOR.fg_str(),
            binary_expression.operator.token.span.literal
        ));
        self.add_whitespace();
        self.visit_expr(ast, binary_expression.right);
    }

    fn visit_parenthesized_expression(
        &mut self,
        ast: &mut Ast,
        parenthesized_expression: &ParenthesizedExpr,
        _expr: &Expr,
    ) {
        self.result
            .push_str(&format!("{}{}", Self::TEXT_COLOR.fg_str(), "("));
        self.visit_expr(ast, parenthesized_expression.expression);
        self.result
            .push_str(&format!("{}{}", Self::TEXT_COLOR.fg_str(), ")"));
    }

    fn visit_variable_expression(
        &mut self,
        _ast: &mut Ast,
        variable_expression: &VariableExpr,
        _expr: &Expr,
    ) {
        self.result.push_str(&format!(
            "{}{}",
            Self::VARIABLE_COLOR.fg_str(),
            variable_expression.identifier.span.literal
        ));
    }
}
