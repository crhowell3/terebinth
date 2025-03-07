//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use compilation_unit::CompilationUnit;

mod ast;
mod compilation_unit;
mod diagnostics;
mod source;

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

fn main() -> Result<(), ()> {
    let args = Args::parse();
    let file_path = args.source_file;
    let file_contents = std::fs::read_to_string(file_path).map_err(|_| ())?;

    let compilation_unit = CompilationUnit::compile(&file_contents).map_err(|_| ())?;
    compilation_unit.run();
    Ok(())
}
