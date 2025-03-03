//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use super::{AstBinaryOperatorKind, AstNumberExpression, AstVisitor, lexer::TextSpan};

pub struct AstEvaluator {
    pub last_value: Option<i64>,
}

impl AstEvaluator {
    pub fn new() -> Self {
        Self { last_value: None }
    }
}

impl AstVisitor for AstEvaluator {
    fn visit_number(&mut self, number: &AstNumberExpression) {
        self.last_value = Some(number.number);
    }

    fn visit_error(&mut self, span: &TextSpan) {
        todo!()
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
        });
    }
}
