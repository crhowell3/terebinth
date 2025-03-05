use termion::color;

use crate::ast::*;

pub struct AstPrinter {
    indent: usize,
    pub result: String,
}

impl AstPrinter {
    const NUMBER_COLOR: color::Magenta = color::Magenta;
    const TEXT_COLOR: color::LightWhite = color::LightWhite;
    const KEYWORD_COLOR: color::Blue = color::Blue;
    const VARIABLE_COLOR: color::Green = color::Green;
    const BOOLEAN_COLOR: color::LightMagenta = color::LightMagenta;

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
            .push_str(&format!("{}{}", Self::BOOLEAN_COLOR.fg_str(), boolean))
    }

    pub fn new() -> Self {
        Self {
            indent: 0,
            result: String::new(),
        }
    }
}

impl AstVisitor<'_> for AstPrinter {
    fn visit_boolean_expression(&mut self, boolean: &AstBooleanExpression) {
        self.add_boolean_literal(boolean.value);
    }

    fn visit_if_statement(&mut self, if_statement: &AstIfStatement) {
        self.add_keyword("if");
        self.add_whitespace();
        self.visit_expression(&if_statement.condition);
        self.add_whitespace();
        self.visit_statement(&if_statement.then_branch);
        if let Some(else_branch) = &if_statement.else_branch {
            self.add_keyword("else");
            self.add_whitespace();
            self.visit_statement(&else_branch.else_statement);
        }
    }

    fn visit_let_statement(&mut self, let_statement: &AstLetStatement) {
        self.add_keyword("let");
        self.add_whitespace();
        self.add_text(let_statement.identifier.span.literal.as_str());
        self.add_whitespace();
        self.add_text("=");
        self.add_whitespace();
        self.visit_expression(&let_statement.initializer);
    }

    fn visit_assignment_expression(&mut self, assignment_expression: &AstAssignmentExpression) {
        self.add_variable(assignment_expression.identifier.span.literal.as_str());
        self.add_whitespace();
        self.add_text("=");
        self.add_whitespace();
        self.visit_expression(&assignment_expression.expression);
    }

    fn visit_statement(&mut self, statement: &AstStatement) {
        self.add_indent();
        Self::do_visit_statement(self, statement);
        self.result.push_str(&format!("{}\n", Fg(Reset)));
    }

    fn visit_block_statement(&mut self, block_statement: &AstBlockStatement) {
        self.add_text("{");
        self.add_newline();
        self.indent += 1;
        for statement in &block_statement.statements {
            self.visit_statement(statement);
        }
        self.indent -= 1;
        self.add_indent();
        self.add_text("}");
    }

    fn visit_number_expression(&mut self, number: &AstNumberExpression) {
        self.result
            .push_str(&format!("{}{}", Self::NUMBER_COLOR.fg_str(), number.number));
    }

    fn visit_error(&mut self, span: &TextSpan) {
        self.result
            .push_str(&format!("{}{}", Self::TEXT_COLOR.fg_str(), span.literal));
    }

    fn visit_unary_expression(&mut self, unary_expression: &AstUnaryExpression) {
        self.result.push_str(&format!(
            "{}{}",
            Self::TEXT_COLOR.fg_str(),
            unary_expression.operator.token.span.literal
        ));
        self.visit_expression(&unary_expression.operand);
    }

    fn visit_binary_expression(&mut self, binary_expression: &AstBinaryExpression) {
        self.visit_expression(&binary_expression.left);
        self.add_whitespace();
        self.result.push_str(&format!(
            "{}{}",
            Self::TEXT_COLOR.fg_str(),
            binary_expression.operator.token.span.literal
        ));
        self.add_whitespace();
        self.visit_expression(&binary_expression.right);
    }

    fn visit_parenthesized_expression(
        &mut self,
        parenthesized_expression: &AstParenthesizedExpression,
    ) {
        self.result
            .push_str(&format!("{}{}", Self::TEXT_COLOR.fg_str(), "("));
        self.visit_expression(&parenthesized_expression.expression);
        self.result
            .push_str(&format!("{}{}", Self::TEXT_COLOR.fg_str(), ")"));
    }

    fn visit_variable_expression(&mut self, variable_expression: &AstVariableExpression) {
        self.result.push_str(&format!(
            "{}{}",
            Self::VARIABLE_COLOR.fg_str(),
            variable_expression.identifier.span.literal
        ));
    }
}
