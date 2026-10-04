//! Untimed preparation declared by individual `.eckb` documents.
//!
//! `>>> prepare` contains one command per line, with quoted arguments supported.
//! `python <file> [arguments...]` selects Python 3 portably. Other commands run
//! directly. Optional `windows:`, `linux:`, `macos:` or comma-separated platform
//! prefixes restrict a command to those hosts. No shell expansion is performed.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[cfg(test)]
#[path = "preparation.tests.rs"]
mod tests;

/// One explicit preparation step with arguments passed directly to its process.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PreparationStep {
    Python {
        file: PathBuf,
        arguments: Vec<String>,
        platforms: Vec<PreparationPlatform>,
    },
    Command {
        program: String,
        arguments: Vec<String>,
        platforms: Vec<PreparationPlatform>,
    },
}

/// Optional host selection for commands that require a particular operating system.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PreparationPlatform {
    Windows,
    Linux,
    Macos,
}

impl PreparationStep {
    /// Checks platform filters while treating an omitted list as portable.
    fn applies_to(&self, operating_system: &str) -> bool {
        let platforms = match self {
            Self::Python { platforms, .. } | Self::Command { platforms, .. } => platforms,
        };
        platforms.is_empty()
            || platforms.iter().any(|platform| {
                matches!(
                    (platform, operating_system),
                    (PreparationPlatform::Windows, "windows")
                        | (PreparationPlatform::Linux, "linux")
                        | (PreparationPlatform::Macos, "macos")
                )
            })
    }
}

/// Converts preparation lines into literal argument vectors and optional host filters.
pub(crate) fn parse_preparation(contents: &str) -> Result<Vec<PreparationStep>, String> {
    let mut steps = Vec::new();
    for line in contents.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut arguments = preparation_arguments(line)?.into_iter();
        let first = arguments.next().ok_or("prepare requires a program")?;
        let (program, platforms) = if let Some(prefix) = first.strip_suffix(':') {
            let platforms = prefix
                .split(',')
                .map(|name| match name {
                    "windows" => Ok(PreparationPlatform::Windows),
                    "linux" => Ok(PreparationPlatform::Linux),
                    "macos" => Ok(PreparationPlatform::Macos),
                    _ => Err(format!("unknown prepare platform `{name}`")),
                })
                .collect::<Result<Vec<_>, _>>()?;
            (
                arguments
                    .next()
                    .ok_or("prepare requires a program after the platform prefix")?,
                platforms,
            )
        } else {
            (first, Vec::new())
        };
        if program.trim().is_empty() {
            return Err("prepare requires a nonempty program".into());
        }
        let step = if program.eq_ignore_ascii_case("python") {
            let file = arguments
                .next()
                .filter(|file| !file.trim().is_empty())
                .ok_or("python preparation requires a script file")?;
            PreparationStep::Python {
                file: file.into(),
                arguments: arguments.collect(),
                platforms,
            }
        } else {
            PreparationStep::Command {
                program,
                arguments: arguments.collect(),
                platforms,
            }
        };
        steps.push(step);
    }
    if steps.is_empty() {
        return Err("prepare must contain at least one step".into());
    }
    Ok(steps)
}

/// Splits literal tokens, preserving Windows paths and spaces within matching quotes.
fn preparation_arguments(line: &str) -> Result<Vec<String>, String> {
    let mut arguments = Vec::new();
    let mut argument = String::new();
    let mut characters = line.chars().peekable();
    let mut quote = None;
    let mut started = false;
    while let Some(character) = characters.next() {
        if quote == Some(character) {
            quote = None;
        } else if quote.is_none() && matches!(character, '\'' | '"') {
            quote = Some(character);
            started = true;
        } else if character == '\\'
            && quote.is_some()
            && characters
                .peek()
                .is_some_and(|next| Some(*next) == quote || *next == '\\')
        {
            argument.push(characters.next().unwrap());
        } else if character.is_whitespace() && quote.is_none() {
            if started {
                arguments.push(std::mem::take(&mut argument));
                started = false;
            }
        } else {
            argument.push(character);
            started = true;
        }
    }
    if quote.is_some() {
        return Err("unterminated quoted prepare argument".into());
    }
    if started {
        arguments.push(argument);
    }
    Ok(arguments)
}

/// Executes ordered preparation once per directory and step, before runtime measurements.
pub(crate) fn prepare_benchmark(
    directory: &Path,
    steps: &[PreparationStep],
    prepared: &mut HashSet<(PathBuf, PreparationStep)>,
) -> Result<(), String> {
    let directory = directory
        .canonicalize()
        .map_err(|error| format!("cannot resolve benchmark directory: {error}"))?;
    for step in steps {
        if !step.applies_to(std::env::consts::OS) {
            continue;
        }
        let mut normalized = step.clone();
        match &mut normalized {
            PreparationStep::Python {
                file, platforms, ..
            } => {
                *file = directory.join(&*file).canonicalize().map_err(|error| {
                    format!("cannot resolve prepare script {}: {error}", file.display())
                })?;
                platforms.clear();
            }
            PreparationStep::Command { platforms, .. } => platforms.clear(),
        }
        let identity = (directory.clone(), normalized.clone());
        if prepared.contains(&identity) {
            continue;
        }
        run_preparation_step(&directory, &normalized)?;
        prepared.insert(identity);
    }
    Ok(())
}

/// Runs one preparation process and forwards its status output outside timed launches.
fn run_preparation_step(directory: &Path, step: &PreparationStep) -> Result<(), String> {
    let mut command = match step {
        PreparationStep::Python {
            file, arguments, ..
        } => {
            let mut command = python_command(std::env::consts::OS)?;
            command.arg(directory.join(file)).args(arguments);
            command
        }
        PreparationStep::Command {
            program, arguments, ..
        } => {
            let program_path = Path::new(program);
            let executable = if program_path.is_relative() && program_path.components().count() > 1
            {
                directory.join(program_path)
            } else {
                program_path.to_path_buf()
            };
            let mut command = Command::new(executable);
            command.args(arguments);
            command
        }
    };
    let name = command.get_program().to_string_lossy().into_owned();
    let output = command
        .current_dir(directory)
        .output()
        .map_err(|error| format!("cannot execute prepare program `{name}`: {error}"))?;
    finish_preparation(&name, output)
}

/// Lists Python 3 entry points, preferring the Windows launcher on Windows.
fn python_candidates(operating_system: &str) -> Vec<(&'static str, &'static [&'static str])> {
    if operating_system == "windows" {
        vec![("py", &["-3"]), ("python3", &[]), ("python", &[])]
    } else {
        vec![("python3", &[]), ("python", &[])]
    }
}

/// Selects an installed Python 3 interpreter before executing a script exactly once.
fn python_command(operating_system: &str) -> Result<Command, String> {
    for (program, arguments) in python_candidates(operating_system) {
        let probe = Command::new(program)
            .args(arguments)
            .args([
                "-c",
                "import sys; sys.exit(0 if sys.version_info.major == 3 else 1)",
            ])
            .output();
        match probe {
            Ok(output) if output.status.success() => {
                let mut command = Command::new(program);
                command.args(arguments);
                return Ok(command);
            }
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("cannot check Python interpreter: {error}")),
        }
    }
    Err("prepare requires an installed Python 3 interpreter".into())
}

/// Displays preparation output and prevents measurement after any failed preparation.
fn finish_preparation(program: &str, output: Output) -> Result<(), String> {
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "prepare program `{program}` failed with status {}",
            output.status
        ))
    }
}
