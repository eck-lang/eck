//! Execution of `.eckt` language tests against the Eck CLI.
//!
//! Each test runs in an isolated temporary directory so concurrent cases and
//! repeated runs never share source files.

use super::{
    cargo::project_root,
    discovery::discover_test_paths,
    eckt::{LanguageTest, parse_language_test},
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command},
};

#[cfg(test)]
#[path = "execution.tests.rs"]
mod tests;

/// A temporary directory removed automatically after a language test completes.
struct TemporaryCaseDirectory {
    path: PathBuf,
}

impl TemporaryCaseDirectory {
    /// Creates a unique temporary directory for one language test execution.
    fn create(case_number: usize) -> Result<Self, String> {
        let base_name = format!("eck-language-test-{}-{case_number}", process::id());

        for attempt in 0..100 {
            let path = env::temp_dir().join(format!("{base_name}-{attempt}"));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!(
                        "cannot create temporary directory `{}`: {error}",
                        path.display()
                    ));
                }
            }
        }

        Err("cannot allocate a unique temporary test directory".into())
    }
}

impl Drop for TemporaryCaseDirectory {
    /// Removes the isolated files created while executing a language test.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Discovers, parses, and executes the `.eckt` test cases below the given
/// roots, returning true when every executed test passes.
///
/// Stops after the first failure so a broken case interrupts the run instead
/// of letting later cases continue.
pub(crate) fn execute_language_tests(
    search_roots: &[PathBuf],
    eck_binary: &Path,
) -> Result<bool, String> {
    let test_paths = discover_test_paths(search_roots)?;
    if test_paths.is_empty() {
        return Err("no `.eckt` language tests found".into());
    }

    let project_root = project_root();
    let mut passed_count = 0;
    let mut failure_count = 0;

    for (case_number, test_path) in test_paths.iter().enumerate() {
        let display_path = test_path.strip_prefix(&project_root).unwrap_or(test_path);
        match execute_test_path(test_path, eck_binary, case_number) {
            Ok(title) => {
                println!("[PASS] {} — {title}", display_path.display());
                passed_count += 1;
            }
            Err(failure) => {
                eprintln!("[FAIL] {}", display_path.display());
                for line in failure.lines() {
                    eprintln!("  {line}");
                }
                failure_count += 1;
                break;
            }
        }
    }
    println!(
        "\n{passed_count} passed; {failure_count} failed; {} total",
        test_paths.len()
    );
    Ok(failure_count == 0)
}

/// Parses and executes one language-test file, returning its title on success.
fn execute_test_path(
    test_path: &Path,
    eck_binary: &Path,
    case_number: usize,
) -> Result<String, String> {
    let contents =
        fs::read_to_string(test_path).map_err(|error| format!("cannot read test: {error}"))?;
    let test = parse_language_test(&contents).map_err(|error| format!("invalid test: {error}"))?;
    let title = test.title.clone();

    if test.configuration.is_some() {
        return Err(format!(
            "{title}\n{}\nconfiguration input is reserved but not supported by the Eck CLI yet",
            test.description
        ));
    }

    let temporary_directory = TemporaryCaseDirectory::create(case_number)?;
    let source_path = temporary_directory.path.join("test.eck");
    fs::write(&source_path, &test.source)
        .map_err(|error| format!("cannot write temporary source: {error}"))?;

    let output = Command::new(eck_binary)
        .arg(&source_path)
        .current_dir(&temporary_directory.path)
        .output()
        .map_err(|error| format!("cannot execute `{}`: {error}", eck_binary.display()))?;

    compare_process_output(&test, output, &source_path.to_string_lossy())?;
    Ok(title)
}

/// Compares one Eck process result against the expected outputs, reporting
/// the first difference for standard output, standard error, or exit code.
fn compare_process_output(
    test: &LanguageTest,
    output: process::Output,
    source_path_text: &str,
) -> Result<(), String> {
    let actual_standard_output = String::from_utf8(output.stdout)
        .map_err(|error| format!("Eck wrote non-UTF-8 standard output: {error}"))?
        .replace(source_path_text, "<test.eck>");
    let actual_standard_error = String::from_utf8(output.stderr)
        .map_err(|error| format!("Eck wrote non-UTF-8 standard error: {error}"))?
        .replace(source_path_text, "<test.eck>");
    let actual_exit_code = output.status.code();

    let mut differences = Vec::new();
    if test.expected_standard_output != actual_standard_output {
        differences.push(format_text_difference(
            "stdout",
            &test.expected_standard_output,
            &actual_standard_output,
        ));
    }
    if test.expected_standard_error != actual_standard_error {
        differences.push(format_text_difference(
            "stderr",
            &test.expected_standard_error,
            &actual_standard_error,
        ));
    }
    if actual_exit_code != Some(test.expected_exit_code) {
        differences.push(format!(
            "exit code differs\nexpected: {}\nactual:   {}",
            test.expected_exit_code,
            actual_exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "terminated by signal".into())
        ));
    }

    if differences.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{}\n{}\n{}",
            test.title,
            test.description,
            differences.join("\n")
        ))
    }
}

/// Formats the first byte-level difference between expected and actual text.
fn format_text_difference(label: &str, expected: &str, actual: &str) -> String {
    let difference_offset = expected
        .bytes()
        .zip(actual.bytes())
        .position(|(expected_byte, actual_byte)| expected_byte != actual_byte)
        .unwrap_or_else(|| expected.len().min(actual.len()));
    let line_number = expected.as_bytes()[..difference_offset.min(expected.len())]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1;

    format!(
        "{label} differs at line {line_number}, byte {difference_offset}\nexpected: {expected:?}\nactual:   {actual:?}"
    )
}
