//! Cross-platform entry point for the complete repository test suite.
//!
//! Without arguments the runner executes every Rust test followed by every
//! `.eckt` language test. With `--binary <eck-binary> <paths>...` it executes
//! one focused `.eckt` subset instead.

use std::{env, process};

use arguments::RunnerArguments;
use cargo::{CargoOutput, default_eck_binary, project_root, run_cargo};
use execution::execute_language_tests;

#[cfg(test)]
#[path = "main.tests.rs"]
mod tests;

mod arguments;
mod cargo;
mod discovery;
mod eckt;
mod execution;

/// Selects the test scope from the raw command-line arguments.
#[derive(Debug, PartialEq, Eq)]
enum Mode {
    /// Runs every Rust test plus every `.eckt` language test.
    Complete,
    /// Runs one focused `.eckt` subset against an explicit binary.
    Focused(RunnerArguments),
}

/// Runs the language-test runner and maps failures to process exit codes.
fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => process::exit(1),
        Err(error) => {
            eprintln!("eck-test-runner: {error}");
            process::exit(2);
        }
    }
}

/// Executes the selected test scope, returning true when everything passes.
fn run() -> Result<bool, String> {
    match select_mode(env::args_os().skip(1))? {
        Mode::Complete => run_all_tests(),
        Mode::Focused(arguments) => {
            execute_language_tests(&arguments.search_roots, &arguments.eck_binary)
        }
    }
}

/// Selects complete or focused testing from the raw command-line arguments.
fn select_mode(arguments: impl Iterator<Item = std::ffi::OsString>) -> Result<Mode, String> {
    let raw_arguments: Vec<std::ffi::OsString> = arguments.collect();
    if raw_arguments.is_empty() {
        return Ok(Mode::Complete);
    }
    arguments::parse_arguments(raw_arguments.into_iter()).map(Mode::Focused)
}

/// Runs the complete test suite: every Rust test followed by every `.eckt`
/// language test below the default roots.
fn run_all_tests() -> Result<bool, String> {
    let project_root = project_root();
    if !run_cargo(&project_root, ["test", "--workspace"], CargoOutput::Stream)? {
        return Ok(false);
    }
    let eck_binary = default_eck_binary(&project_root)?;
    let search_roots = [
        project_root.join("testing").join("use-cases"),
        project_root.join("testing").join("regressions"),
    ];
    execute_language_tests(&search_roots, &eck_binary)
}
