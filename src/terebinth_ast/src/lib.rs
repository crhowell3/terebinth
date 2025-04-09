//! Terebinth abstract syntax tree (AST)

#![allow(internal_features)]
#![feature(associated_type_defaults)]
#![feature(box_patterns)]
#![feature(if_let_guard)]
#![feature(let_chains)]
#![feature(negative_impls)]
#![feature(never_type)]
#![feature(rustdoc_internals)]
#![feature(stmt_expr_attributes)]

pub mod util {
    pub mod case;
    pub mod classify;
    pub mod comments;
    pub mod literal;
    pub mod parser;
    pub mod unicode;
}

pub mod ast;
pub mod attr;
pub mod entry;
pub mod expand;
pub mod ir;
pub mod mut_visit;
pub mod node_id;
pub mod token;
pub mod visit;

pub use self::ast::*;
