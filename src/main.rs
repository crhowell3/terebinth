//! The Terebinth compiler.

use std::panic::{self, PanicHookInfo, catch_unwind};
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

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

pub fn catch_fatal_errors<F: FnOnce() -> R, R>(f: F) -> Result<R, FatalError> {
    catch_unwind(panic::AssertUnwindSafe(f)).map_err(|value| {
        if value.is::<errors::FatalErrorMarker>() {
            FatalError
        } else {
            panic::resume_unwind(value);
        }
    })
}

pub fn catch_with_exit_code(f: impl FnOnce()) -> i32 {
    match catch_fatal_errors(f) {
        Ok(()) => EXIT_SUCCESS,
        _ => EXIT_FAILURE,
    }
}

pub fn main() -> ! {
    let args = Args::parse();
    let file_path = args.source_file;
    let file_contents = std::fs::read_to_string(file_path).map_err(|_| ())?;

    let early_dcx = EarlyDiagCtx::new(ErrorOutputType::default());

    let mut callbacks = TimePassesCallbacks::default();
    let exit_code =
        catch_with_exit_code(|| run_compiler(&args::raw_args(&early_dcx), &mut callbacks));

    process::exit(exit_code)
}
