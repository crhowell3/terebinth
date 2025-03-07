//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::collections::HashMap;

use crate::compilation_unit::GlobalScope;

use super::{
    AstBinaryOperatorKind, AstIfStatement, AstLetStatement, AstNumberExpression,
    AstParenthesizedExpression, AstUnaryOperatorKind, AstVariableExpression, AstVisitor,
    lexer::TextSpan,
};

pub struct Frame {
    variables: HashMap<String, i64>,
}

impl Frame {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    fn insert(&mut self, identifier: String, value: i64) {
        self.variables.insert(identifier, value);
    }

    fn get(&self, identifier: &String) -> Option<&i64> {
        self.variables.get(identifier)
    }
}

pub struct Frames {
    frames: Vec<Frame>,
}

impl Frames {
    fn new() -> Self {
        Self {
            frames: vec![Frame::new()],
        }
    }

    fn push(&mut self) {
        self.frames.push(Frame::new());
    }

    fn pop(&mut self) {
        self.frames.pop();
    }

    fn update(&mut self, identifier: String, value: i64) {
        for frame in self.frames.iter_mut().rev() {
            if frame.variables.contains_key(&identifier) {
                frame.insert(identifier, value);
                return;
            }
        }
        panic!("Variable {} not found", identifier)
    }

    fn insert(&mut self, identifier: String, value: i64) {
        self.frames.last_mut().unwrap().insert(identifier, value);
    }

    fn get(&self, identifier: &String) -> Option<&i64> {
        for frame in self.frames.iter().rev() {
            if let Some(value) = frame.get(identifier) {
                return Some(value);
            }
        }
        None
    }
}

pub struct AstEvaluator<'a> {
    pub last_value: Option<i64>,
    pub frames: Frames,
    pub global_scope: &'a GlobalScope,
}

impl<'a> AstEvaluator<'a> {
    pub fn new(global_scope: &'a GlobalScope) -> Self {
        Self {
            last_value: None,
            frames: Frames::new(),
            global_scope,
        }
    }

    fn eval_boolean_operation<F>(&self, instruction: F) -> i64
    where
        F: FnOnce() -> bool,
    {
        let result = instruction();
        if result { 1 } else { 0 }
    }

    fn push_frame(&mut self) {
        self.frames.push();
    }

    fn pop_frame(&mut self) {
        self.frames.pop();
    }
}

impl<'a> AstVisitor<'_> for AstEvaluator<'a> {
    fn visit_func_decl_statement(&mut self, func_decl_statement: &super::AstFuncDeclStatement) {}

    fn visit_if_statement(&mut self, if_statement: &AstIfStatement) {
        self.push_frame();
        self.visit_expression(&if_statement.condition);
        if self.last_value.unwrap() != 0 {
            self.push_frame();
            self.visit_statement(&if_statement.then_branch);
            self.pop_frame();
        } else if let Some(else_branch) = &if_statement.else_branch {
            self.push_frame();
            self.visit_statement(&else_branch.else_statement);
            self.pop_frame();
        }
        self.pop_frame();
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

    fn visit_while_statement(&mut self, while_statement: &super::AstWhileStatement) {
        self.push_frame();
        self.visit_expression(&while_statement.condition);
        while self.last_value.unwrap() != 0 {
            self.visit_statement(&while_statement.body);
            self.visit_expression(&while_statement.condition);
        }
        self.pop_frame();
    }

    fn visit_block_statement(&mut self, block_statement: &super::AstBlockStatement) {
        self.push_frame();
        for statement in &block_statement.statements {
            self.visit_statement(statement);
        }
        self.pop_frame();
    }

    fn visit_let_statement(&mut self, let_statement: &AstLetStatement) {
        self.visit_expression(&let_statement.initializer);
        self.frames.insert(
            let_statement.identifier.span.literal.clone(),
            self.last_value.unwrap(),
        );
    }

    fn visit_variable_expression(&mut self, variable_expression: &AstVariableExpression) {
        let identifier = &variable_expression.identifier.span.literal;
        self.last_value = Some(
            *self
                .frames
                .get(identifier)
                .unwrap_or_else(|| panic!("Variable {} not found", identifier)),
        );
    }

    fn visit_call_expression(&mut self, call_expression: &super::AstCallExpression) {
        let function = self
            .global_scope
            .lookup_function(&call_expression.identifier.span.literal)
            .unwrap();
        let mut arguments = Vec::new();
        for argument in &call_expression.arguments {
            self.visit_expression(argument);
            arguments.push(self.last_value.unwrap());
        }
        self.push_frame();
        for (i, argument) in arguments.iter().enumerate() {
            let parameter_name = function.parameters[i].clone();
            self.frames.insert(parameter_name, *argument);
        }

        self.visit_statement(&function.body);
        self.pop_frame();
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
        self.frames
            .update(identifier.clone(), self.last_value.unwrap());
    }

    fn visit_boolean_expression(&mut self, boolean: &super::AstBooleanExpression) {
        self.last_value = Some(boolean.value as i64);
    }
}
