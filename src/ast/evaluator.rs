//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::collections::HashMap;

use crate::compilation_unit::{GlobalScope, VariableIndex};

use super::{
    AssignmentExpr, Ast, BinaryExpr, BinaryOperatorKind, BlockExpr, BooleanExpr, CallExpr, Expr,
    FunctionDeclaration, IfExpr, LetStmt, NumberExpr, ParenthesizedExpr, Stmt, UnaryExpr,
    UnaryOperatorKind, VariableExpr, Visitor, WhileStmt,
};
use crate::source::span::TextSpan;

pub struct Frame {
    variables: HashMap<VariableIndex, i64>,
}

impl Frame {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    fn insert(&mut self, idx: VariableIndex, value: i64) {
        self.variables.insert(idx, value);
    }

    fn get(&self, idx: VariableIndex) -> Option<&i64> {
        self.variables.get(&idx)
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

    fn update(&mut self, idx: VariableIndex, value: i64) {
        for frame in self.frames.iter_mut().rev() {
            if frame.variables.contains_key(&idx) {
                frame.insert(idx, value);
                return;
            }
        }
    }

    fn insert(&mut self, idx: VariableIndex, value: i64) {
        self.frames.last_mut().unwrap().insert(idx, value);
    }

    fn get(&self, idx: VariableIndex) -> Option<&i64> {
        for frame in self.frames.iter().rev() {
            if let Some(value) = frame.get(idx) {
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

    fn eval_boolean_operation<F>(instruction: F) -> i64
    where
        F: FnOnce() -> bool,
    {
        let result = instruction();
        i64::from(result)
    }

    fn push_frame(&mut self) {
        self.frames.push();
    }

    fn pop_frame(&mut self) {
        self.frames.pop();
    }
}

impl Visitor for AstEvaluator<'_> {
    fn visit_func_decl_stmt(&mut self, _ast: &mut Ast, _func_decl_statement: &FunctionDeclaration) {
    }

    fn visit_if_expr(&mut self, ast: &mut Ast, if_statement: &IfExpr, _expr: &Expr) {
        self.push_frame();
        self.visit_expr(ast, if_statement.condition);
        if self.last_value.unwrap() != 0 {
            self.push_frame();
            self.visit_expr(ast, if_statement.then_branch);
            self.pop_frame();
        } else if let Some(else_branch) = &if_statement.else_branch {
            self.push_frame();
            self.visit_expr(ast, else_branch.expr);
            self.pop_frame();
        }
        self.pop_frame();
    }

    fn visit_number_expr(&mut self, _ast: &mut Ast, number: &NumberExpr, _expr: &Expr) {
        self.last_value = Some(number.number);
    }

    fn visit_error(&mut self, _ast: &mut Ast, _span: &TextSpan) {
        todo!()
    }

    fn visit_unary_expr(&mut self, ast: &mut Ast, unary_expression: &UnaryExpr, _expr: &Expr) {
        self.visit_expr(ast, unary_expression.operand);
        let operand = self.last_value.unwrap();
        self.last_value = Some(match unary_expression.operator.kind {
            UnaryOperatorKind::Minus => -operand,
            UnaryOperatorKind::BitwiseNot => !operand,
        });
    }

    fn visit_binary_expr(&mut self, ast: &mut Ast, binary_expr: &BinaryExpr, _expr: &Expr) {
        self.visit_expr(ast, binary_expr.left);
        let left = self.last_value.unwrap();
        self.visit_expr(ast, binary_expr.right);
        let right = self.last_value.unwrap();
        self.last_value = Some(match binary_expr.operator.kind {
            BinaryOperatorKind::Plus => left + right,
            BinaryOperatorKind::Minus => left - right,
            BinaryOperatorKind::Multiply => left * right,
            BinaryOperatorKind::Divide => left / right,
            BinaryOperatorKind::BitwiseAnd => left & right,
            BinaryOperatorKind::BitwiseOr => left | right,
            BinaryOperatorKind::Power => {
                left.pow(u32::try_from(right).expect("Exponent larger than u32"))
            }
            BinaryOperatorKind::BitwiseXor => left ^ right,
            BinaryOperatorKind::LeftShift => left << right,
            BinaryOperatorKind::RightShift => left >> right,
            BinaryOperatorKind::Equals => Self::eval_boolean_operation(|| left == right),
            BinaryOperatorKind::NotEquals => Self::eval_boolean_operation(|| left != right),
            BinaryOperatorKind::LessThan => Self::eval_boolean_operation(|| left < right),
            BinaryOperatorKind::LessThanOrEqual => Self::eval_boolean_operation(|| left <= right),
            BinaryOperatorKind::GreaterThan => Self::eval_boolean_operation(|| left > right),
            BinaryOperatorKind::GreaterThanOrEqual => {
                Self::eval_boolean_operation(|| left >= right)
            }
        });
    }

    fn visit_while_stmt(&mut self, ast: &mut Ast, while_statement: &WhileStmt) {
        self.push_frame();
        self.visit_expr(ast, while_statement.condition);
        while self.last_value.unwrap() != 0 {
            self.visit_expr(ast, while_statement.body);
            self.visit_expr(ast, while_statement.condition);
        }
        self.pop_frame();
    }

    fn visit_block_expr(&mut self, ast: &mut Ast, block_statement: &BlockExpr, _expr: &Expr) {
        self.push_frame();
        for statement in &block_statement.stmts {
            self.visit_stmt(ast, *statement);
        }
        self.pop_frame();
    }

    fn visit_let_stmt(&mut self, ast: &mut Ast, let_statement: &LetStmt, _stmt: &Stmt) {
        self.visit_expr(ast, let_statement.initializer);
        self.frames
            .insert(let_statement.variable_idx, self.last_value.unwrap());
    }

    fn visit_variable_expr(
        &mut self,
        _ast: &mut Ast,
        variable_expression: &VariableExpr,
        _expr: &Expr,
    ) {
        let identifier = &variable_expression.identifier.span.literal;
        self.last_value = Some(
            *self
                .frames
                .get(variable_expression.variable_idx)
                .unwrap_or_else(|| panic!("Variable {identifier} not found")),
        );
    }

    fn visit_call_expr(&mut self, ast: &mut Ast, call_expression: &CallExpr, _expr: &Expr) {
        let function_idx = self
            .global_scope
            .lookup_function(&call_expression.identifier.span.literal)
            .unwrap();
        let function = self.global_scope.functions.get(function_idx);
        let mut arguments = Vec::new();
        for argument in &call_expression.arguments {
            self.visit_expr(ast, *argument);
            arguments.push(self.last_value.unwrap());
        }
        self.push_frame();
        for (argument, param) in arguments.iter().zip(function.parameters.iter()) {
            self.frames.insert(*param, *argument);
        }

        self.visit_stmt(ast, function.body);
        self.pop_frame();
    }

    fn visit_parenthesized_expr(
        &mut self,
        ast: &mut Ast,
        parenthesized_expression: &ParenthesizedExpr,
        _expr: &Expr,
    ) {
        self.visit_expr(ast, parenthesized_expression.expression);
    }

    fn visit_assignment_expr(
        &mut self,
        ast: &mut Ast,
        assignment_expression: &AssignmentExpr,
        _expr: &Expr,
    ) {
        self.visit_expr(ast, assignment_expression.expression);
        self.frames
            .update(assignment_expression.variable_idx, self.last_value.unwrap());
    }

    fn visit_boolean_expr(&mut self, _ast: &mut Ast, boolean: &BooleanExpr, _expr: &Expr) {
        self.last_value = Some(i64::from(boolean.value));
    }
}
