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
        let a = 0
        let b = 1
        if b > a
            a = 10
        else
            a = 5
        
        a
    ";

    let compilation_unit = CompilationUnit::compile(input);
    compilation_unit.maybe_run();
}
