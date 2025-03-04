//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

use compilation_unit::CompilationUnit;

mod ast;
mod compilation_unit;
mod diagnostics;
mod source;

fn main() {
    let input = "\
        let a = (1 + 2) * 3
    ";

    let compilation_unit = CompilationUnit::compile(input);
    compilation_unit.maybe_run();
}
