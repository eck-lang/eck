//! Command-line parsing for focused language-test runs.

use std::path::PathBuf;

#[cfg(test)]
#[path = "arguments.tests.rs"]
mod tests;

/// Command-line values required to discover and execute language tests.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RunnerArguments {
    pub(crate) eck_binary: PathBuf,
    pub(crate) search_roots: Vec<PathBuf>,
}

/// Parses the runner's `--binary <path> <test-roots>...` command line.
pub(crate) fn parse_arguments(
    mut arguments: impl Iterator<Item = std::ffi::OsString>,
) -> Result<RunnerArguments, String> {
    let flag = arguments.next().ok_or_else(usage)?;
    if flag != "--binary" {
        return Err(usage());
    }

    let eck_binary = arguments.next().ok_or_else(usage).map(PathBuf::from)?;
    let eck_binary = if eck_binary.is_relative() && eck_binary.components().count() > 1 {
        std::env::current_dir()
            .map_err(|error| format!("cannot resolve the Eck binary path: {error}"))?
            .join(eck_binary)
    } else {
        eck_binary
    };
    let search_roots: Vec<_> = arguments.map(PathBuf::from).collect();
    if search_roots.is_empty() {
        return Err(usage());
    }

    Ok(RunnerArguments {
        eck_binary,
        search_roots,
    })
}

/// Returns the accepted command-line shape for malformed invocations.
///
/// The arguments are optional: without them the runner executes the complete
/// test suite instead of a focused subset.
fn usage() -> String {
    "usage: eck-test-runner [--binary <eck-binary> <test-file-or-directory>...]".into()
}
