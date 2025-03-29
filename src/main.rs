//! The Terebinth compiler.

#![allow(internal_features)]
#![feature(core_intrinsics)]
#![feature(decl_macro)]
#![feature(dropck_eyepatch)]
#![feature(negative_impls)]

use std::path::PathBuf;

use crate::compilation_unit::CompilationUnit;
use anyhow::Result;
use clap::Parser;

mod arena;
mod ast;
mod compilation_unit;
mod lexer;
mod llvm_codegen;
mod parse;
mod span;
mod typings;

#[derive(Parser, Debug)]
#[clap(name = "terebinth")]
#[command(version, about)]
struct Args {
    #[arg(value_parser = check_extension)]
    source_file: PathBuf,
}

fn check_extension(file_path: &str) -> Result<PathBuf, String> {
    let file_path = PathBuf::from(file_path);
    let extension = file_path.extension().ok_or("No file extension")?;
    if extension != "ter" {
        return Err(format!(
            "Invalid file extension: {} (expected .ter)",
            extension.to_string_lossy()
        ));
    }
    Ok(file_path)
}

pub fn main() -> Result<(), ()> {
    let args = Args::parse();
    let file_path = args.source_file;
    let file_contents = std::fs::read_to_string(file_path).map_err(|_| ())?;

    let mut compilation_unit = CompilationUnit::compile(&file_contents).map_err(|_| ())?;
    compilation_unit.run();
    Ok(())
}
