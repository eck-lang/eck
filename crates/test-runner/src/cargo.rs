//! Helpers for orchestrating `cargo` invocations from the runner.

use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(test)]
#[path = "cargo.tests.rs"]
mod tests;

/// Selects how a spawned `cargo` invocation handles its standard output.
pub(crate) enum CargoOutput {
    /// Streams the child output live for user-facing orchestration.
    Stream,
    /// Captures the child output so tests stay silent.
    ///
    /// Constructed only by unit tests, which the unused-code lint cannot see.
    #[allow(dead_code)]
    Capture,
}

/// Returns the repository root derived from this crate's manifest location.
pub(crate) fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the test runner must live below the project root")
        .to_path_buf()
}

/// Returns the Eck binary used when none is supplied explicitly, honouring
/// `ECK_BIN` and otherwise building `eck-cli` like `testing/run.sh` does.
pub(crate) fn default_eck_binary(project_root: &Path) -> Result<PathBuf, String> {
    if let Ok(binary) = env::var("ECK_BIN")
        && !binary.is_empty()
    {
        return Ok(PathBuf::from(binary));
    }
    if !run_cargo(
        project_root,
        ["build", "--quiet", "-p", "eck-cli"],
        CargoOutput::Stream,
    )? {
        return Err("cannot build the Eck CLI".into());
    }
    Ok(project_root
        .join("target")
        .join("debug")
        .join(format!("eck{}", env::consts::EXE_SUFFIX)))
}

/// Runs one `cargo` invocation below the project root, returning true when it
/// succeeds and false when the invoked command reports failure.
pub(crate) fn run_cargo<I, S>(
    project_root: &Path,
    arguments: I,
    output: CargoOutput,
) -> Result<bool, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let arguments: Vec<String> = arguments
        .into_iter()
        .map(|argument| argument.as_ref().to_string_lossy().into_owned())
        .collect();
    let command_description = format!("cargo {}", arguments.join(" "));
    let mut command = Command::new("cargo");
    command.args(&arguments).current_dir(project_root);
    let succeeded = match output {
        CargoOutput::Stream => command
            .status()
            .map_err(|error| format!("cannot execute `{command_description}`: {error}"))?
            .success(),
        CargoOutput::Capture => command
            .output()
            .map_err(|error| format!("cannot execute `{command_description}`: {error}"))?
            .status
            .success(),
    };
    Ok(succeeded)
}
