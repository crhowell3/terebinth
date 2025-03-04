//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::collections::HashMap;

use super::{
    AstBinaryOperatorKind, AstLetStatement, AstNumberExpression, AstParenthesizedExpression,
    AstUnaryOperatorKind, AstVariableExpression, AstVisitor, lexer::TextSpan,
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
}

impl AstVisitor for AstEvaluator {
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
}
