//! Command-line parsing for focused language-test and benchmark runs.

use std::path::PathBuf;

#[cfg(test)]
#[path = "arguments.tests.rs"]
mod tests;

/// Command-line values required to discover and execute language tests.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RunnerArguments {
    pub(crate) eck_binary: Option<PathBuf>,
    pub(crate) search_roots: Vec<PathBuf>,
}

/// Command-line values required to discover and execute Eck benchmarks.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BenchmarkArguments {
    pub(crate) search_roots: Vec<PathBuf>,
}

/// Parses the runner's `[--binary <path>] <test-roots>...` command line.
///
/// Without `--binary` the caller resolves the default binary, honouring
/// `ECK_BIN` and otherwise building `eck-cli`.
pub(crate) fn parse_arguments(
    arguments: impl Iterator<Item = std::ffi::OsString>,
) -> Result<RunnerArguments, String> {
    let mut raw: Vec<std::ffi::OsString> = arguments.collect();
    let eck_binary = if raw.first().is_some_and(|flag| flag == "--binary") {
        raw.remove(0);
        if raw.is_empty() {
            return Err(usage());
        }
        Some(resolve_binary(PathBuf::from(raw.remove(0)))?)
    } else {
        None
    };
    let search_roots: Vec<_> = raw.into_iter().map(PathBuf::from).collect();
    if search_roots.is_empty() {
        return Err(usage());
    }

    Ok(RunnerArguments {
        eck_binary,
        search_roots,
    })
}

/// Parses the benchmark command's `<benchmark-roots>...` arguments.
///
/// An optional separator is accepted because Cargo aliases can forward it to
/// the executable after the `benchmark` subcommand.
pub(crate) fn parse_benchmark_arguments(
    arguments: impl Iterator<Item = std::ffi::OsString>,
) -> Result<BenchmarkArguments, String> {
    let mut search_roots: Vec<_> = arguments.map(PathBuf::from).collect();
    if search_roots
        .first()
        .is_some_and(|argument| argument.as_os_str() == "--")
    {
        search_roots.remove(0);
    }
    if search_roots.is_empty() {
        return Err(benchmark_usage());
    }

    Ok(BenchmarkArguments { search_roots })
}

/// Resolves a relative binary path against the current directory.
fn resolve_binary(eck_binary: PathBuf) -> Result<PathBuf, String> {
    if eck_binary.is_relative() && eck_binary.components().count() > 1 {
        std::env::current_dir()
            .map_err(|error| format!("cannot resolve the Eck binary path: {error}"))
            .map(|current| current.join(eck_binary))
    } else {
        Ok(eck_binary)
    }
}

/// Returns the accepted command-line shape for malformed invocations.
///
/// The arguments are optional: without them the runner executes the complete
/// test suite instead of a focused subset.
fn usage() -> String {
    "usage: eck-test-runner [--binary <eck-binary>] <test-file-or-directory>...".into()
}

/// Returns the accepted command-line shape for malformed benchmark invocations.
fn benchmark_usage() -> String {
    "usage: eck-test-runner benchmark <benchmark-file-or-directory>...".into()
}
