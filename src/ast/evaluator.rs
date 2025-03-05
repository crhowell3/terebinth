//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::collections::HashMap;

use super::{
    AstBinaryOperatorKind, AstIfStatement, AstLetStatement, AstNumberExpression,
    AstParenthesizedExpression, AstUnaryOperatorKind, AstVariableExpression, AstVisitor,
    lexer::TextSpan,
};

pub struct AstEvaluator {
    pub last_value: Option<i64>,
    pub variables: HashMap<String, i64>,
}

impl AstEvaluator {
    pub fn new() -> Self {
        Self {
            last_value: None,
            variables: HashMap::new(),
        }
    }
    fn eval_boolean_operation<F>(&self, instruction: F) -> i64
    where
        F: FnOnce() -> bool,
    {
        let result = instruction();
        if result { 1 } else { 0 }
    }
}

impl AstVisitor for AstEvaluator {
    fn visit_if_statement(&mut self, if_statement: &AstIfStatement) {
        self.visit_expression(&if_statement.condition);
        if self.last_value.unwrap() != 0 {
            self.visit_statement(&if_statement.then_branch);
        } else if let Some(else_branch) = &if_statement.else_branch {
            self.visit_statement(&else_branch.else_statement);
        }
    }

    fn visit_number_expression(&mut self, number: &AstNumberExpression) {
        self.last_value = Some(number.number);
    }

    fn visit_error(&mut self, span: &TextSpan) {
        todo!()
    }

    fn visit_unary_expression(&mut self, unary_expression: &super::AstUnaryExpression) {
        self.visit_expression(&unary_expression.operand);
        let operand = self.last_value.unwrap();
        self.last_value = Some(match unary_expression.operator.kind {
            AstUnaryOperatorKind::Minus => -operand,
            AstUnaryOperatorKind::BitwiseNot => !operand,
        });
    }

    fn visit_binary_expression(&mut self, expr: &super::AstBinaryExpression) {
        self.visit_expression(&expr.left);
        let left = self.last_value.unwrap();
        self.visit_expression(&expr.right);
        let right = self.last_value.unwrap();
        self.last_value = Some(match expr.operator.kind {
            AstBinaryOperatorKind::Plus => left + right,
            AstBinaryOperatorKind::Minus => left - right,
            AstBinaryOperatorKind::Multiply => left * right,
            AstBinaryOperatorKind::Divide => left / right,
            AstBinaryOperatorKind::BitwiseAnd => left & right,
            AstBinaryOperatorKind::BitwiseOr => left | right,
            AstBinaryOperatorKind::Power => left.pow(right as u32),
            AstBinaryOperatorKind::BitwiseXor => left ^ right,
            AstBinaryOperatorKind::LeftShift => left << right,
            AstBinaryOperatorKind::RightShift => left >> right,
            AstBinaryOperatorKind::Equals => self.eval_boolean_operation(|| left == right),
            AstBinaryOperatorKind::NotEquals => self.eval_boolean_operation(|| left != right),
            AstBinaryOperatorKind::LessThan => self.eval_boolean_operation(|| left < right),
            AstBinaryOperatorKind::LessThanOrEqual => self.eval_boolean_operation(|| left <= right),
            AstBinaryOperatorKind::GreaterThan => self.eval_boolean_operation(|| left > right),
            AstBinaryOperatorKind::GreaterThanOrEqual => {
                self.eval_boolean_operation(|| left >= right)
            }
        });
    }

    fn visit_let_statement(&mut self, let_statement: &AstLetStatement) {
        self.visit_expression(&let_statement.initializer);
        self.variables.insert(
            let_statement.identifier.span.literal.clone(),
            self.last_value.unwrap(),
        );
    }

    fn visit_variable_expression(&mut self, variable_expression: &AstVariableExpression) {
        self.last_value = Some(
            *self
                .variables
                .get(&variable_expression.identifier.span.literal)
                .unwrap(),
        );
    }

    fn visit_parenthesized_expression(
        &mut self,
        parenthesized_expression: &AstParenthesizedExpression,
    ) {
        self.visit_expression(&parenthesized_expression.expression);
    }

    fn visit_assignment_expression(
        &mut self,
        assignment_expression: &super::AstAssignmentExpression,
    ) {
        let identifier = &assignment_expression.identifier.span.literal;
        self.visit_expression(&assignment_expression.expression);
        self.variables
            .insert(identifier.clone(), self.last_value.unwrap());
    }
}
