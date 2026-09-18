//! The `rustille` command-line interface.
//!
//! Rendered output goes to standard output (or `--output`); diagnostics go to
//! standard error. The exit code is `0` on success, `1` on a runtime failure
//! and `2` when the arguments themselves are wrong (clap's convention).

mod cli;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let args = cli::Cli::parse();
    match cli::run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rustille: {error:#}");
            ExitCode::from(cli::EXIT_FAILURE)
        }
    }
}
