//! Execution of historical Eck benchmarks.
//!
//! A benchmark always uses the source parsed from the current benchmark file.
//! Runtime versions are built in detached temporary Git worktrees, so running
//! a benchmark never checks out or otherwise changes the caller's checkout.

use super::{
    cargo::project_root,
    discovery::discover_benchmark_paths,
    eckb::{BenchmarkCheckpoint, BenchmarkDefinition, parse_benchmark},
};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

#[cfg(test)]
#[path = "benchmark.tests.rs"]
mod tests;

/// Number of untimed launches used to warm each historical runtime.
const WARMUP_RUNS: usize = 2;

/// Number of timed launches used to calculate one benchmark median.
const MEASURED_RUNS: usize = 7;

/// Maximum number of temporary-name collisions tolerated in one allocation.
const TEMPORARY_PATH_ATTEMPTS: usize = 100;

/// Maximum length of a command failure detail displayed in a result row.
const MAX_ERROR_DETAIL_LENGTH: usize = 120;

/// Counter used to keep temporary benchmark paths unique within one process.
static TEMPORARY_PATH_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// An Eck runtime binary built for one resolved commit.
struct BuiltRuntime {
    worktree: TemporaryWorktree,
    binary_path: PathBuf,
}

/// A cached runtime build, including failures so a bad commit is not rebuilt.
enum CachedRuntime {
    Built(BuiltRuntime),
    Failed(String),
}

/// A detached Git worktree owned by the benchmark run.
struct TemporaryWorktree {
    repository_root: PathBuf,
    container: PathBuf,
    path: PathBuf,
}

impl TemporaryWorktree {
    /// Adds a detached worktree for one commit below a unique temporary path.
    fn create(repository_root: &Path, commit: &str) -> Result<Self, String> {
        let container = reserve_temporary_directory("eck-benchmark-worktree")?;
        let path = container.join("checkout");
        let output = match Command::new("git")
            .arg("-C")
            .arg(repository_root)
            .args(["worktree", "add", "--detach"])
            .arg(&path)
            .arg(commit)
            .output()
        {
            Ok(output) => output,
            Err(error) => {
                cleanup_worktree(&container, repository_root, &path);
                return Err(format!("cannot execute git worktree add: {error}"));
            }
        };
        if !output.status.success() {
            cleanup_worktree(&container, repository_root, &path);
            return Err(format_command_failure("git worktree add", &output));
        }

        Ok(Self {
            repository_root: repository_root.to_path_buf(),
            container,
            path,
        })
    }
}

impl Drop for TemporaryWorktree {
    /// Removes the exact temporary worktree path, even after an execution error.
    fn drop(&mut self) {
        cleanup_worktree(&self.container, &self.repository_root, &self.path);
    }
}

/// A temporary Eck source shared by every runtime in one benchmark file.
struct TemporarySource {
    directory: PathBuf,
    path: PathBuf,
}

impl TemporarySource {
    /// Writes one benchmark source to a unique temporary Eck file.
    fn create(source: &str) -> Result<Self, String> {
        let directory = reserve_temporary_directory("eck-benchmark-source")?;
        let path = directory.join("benchmark.eck");
        if let Err(error) = fs::write(&path, source) {
            remove_exact_path(&directory);
            return Err(format!(
                "cannot write benchmark source {}: {error}",
                path.display()
            ));
        }
        Ok(Self { directory, path })
    }
}

impl Drop for TemporarySource {
    /// Removes the temporary source directory after the benchmark file ends.
    fn drop(&mut self) {
        remove_exact_path(&self.directory);
    }
}

/// Discovers, parses, and benchmarks every `.eckb` file below the roots.
///
/// Every checkpoint and HEAD is resolved to a commit and built in a cached
/// detached worktree. A failed reference, build, or process run is reported as
/// an error row and makes the returned result false, while later rows continue.
pub(crate) fn execute_benchmarks(search_roots: &[PathBuf]) -> Result<bool, String> {
    let benchmark_paths = discover_benchmark_paths(search_roots)?;
    if benchmark_paths.is_empty() {
        return Err("no .eckb benchmarks found".into());
    }

    let repository_root = project_root();
    let mut runtime_cache = HashMap::new();
    let mut all_succeeded = true;

    for benchmark_path in benchmark_paths {
        let contents = fs::read_to_string(&benchmark_path)
            .map_err(|error| format!("cannot read benchmark: {error}"))?;
        let benchmark =
            parse_benchmark(&contents).map_err(|error| format!("invalid benchmark: {error}"))?;
        if !execute_benchmark(
            &benchmark_path,
            &benchmark,
            &repository_root,
            &mut runtime_cache,
        )? {
            all_succeeded = false;
        }
    }

    Ok(all_succeeded)
}

/// Executes one parsed benchmark while preserving its checkpoint order.
fn execute_benchmark(
    benchmark_path: &Path,
    benchmark: &BenchmarkDefinition,
    repository_root: &Path,
    runtime_cache: &mut HashMap<String, CachedRuntime>,
) -> Result<bool, String> {
    let source = TemporarySource::create(&benchmark.source)?;
    let display_path = benchmark_path
        .strip_prefix(repository_root)
        .unwrap_or(benchmark_path);

    println!(
        "\n{} — {}\n{}",
        display_path.display(),
        benchmark.title,
        benchmark.description
    );
    println!(
        "{:<24} {:<12} {:>14} {:>12}",
        "checkpoint", "commit", "median", "delta"
    );

    let mut previous_measurement = None;
    let mut benchmark_succeeded = true;
    for checkpoint in &benchmark.checkpoints {
        let result = measure_checkpoint(checkpoint, repository_root, &source.path, runtime_cache);
        println!(
            "{}",
            render_measurement_row(checkpoint, &result, previous_measurement)
        );
        match result {
            Ok(measurement) => previous_measurement = Some(measurement),
            Err(_) => {
                benchmark_succeeded = false;
                previous_measurement = None;
            }
        }
    }

    let head_checkpoint = BenchmarkCheckpoint {
        name: "HEAD".into(),
        git_reference: "HEAD".into(),
        annotation: None,
        comment: None,
    };
    let result = measure_checkpoint(
        &head_checkpoint,
        repository_root,
        &source.path,
        runtime_cache,
    );
    println!(
        "{}",
        render_measurement_row(&head_checkpoint, &result, previous_measurement)
    );
    if result.is_err() {
        benchmark_succeeded = false;
    }

    Ok(benchmark_succeeded)
}

/// Stores one successful runtime measurement in nanoseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Measurement {
    commit: [u8; 7],
    elapsed_nanoseconds: u128,
}

/// Resolves, builds, warms, and measures one checkpoint.
fn measure_checkpoint(
    checkpoint: &BenchmarkCheckpoint,
    repository_root: &Path,
    source_path: &Path,
    runtime_cache: &mut HashMap<String, CachedRuntime>,
) -> Result<Measurement, String> {
    let resolved_commit = resolve_commit(repository_root, &checkpoint.git_reference)?;
    let commit_key = resolved_commit.clone();
    let runtime = runtime_cache.entry(commit_key.clone()).or_insert_with(|| {
        match build_runtime(repository_root, &resolved_commit) {
            Ok(runtime) => CachedRuntime::Built(runtime),
            Err(error) => CachedRuntime::Failed(error),
        }
    });
    let CachedRuntime::Built(runtime) = runtime else {
        let CachedRuntime::Failed(error) = runtime else {
            unreachable!();
        };
        return Err(format!("unsupported commit {commit_key}: {error}"));
    };

    for _ in 0..WARMUP_RUNS {
        run_once(&runtime.binary_path, &runtime.worktree.path, source_path)?;
    }

    let mut samples = Vec::with_capacity(MEASURED_RUNS);
    for _ in 0..MEASURED_RUNS {
        samples.push(run_once(
            &runtime.binary_path,
            &runtime.worktree.path,
            source_path,
        )?);
    }
    samples.sort_unstable();
    let elapsed_nanoseconds = samples[samples.len() / 2];

    let mut commit = [b' '; 7];
    for (destination, source) in commit.iter_mut().zip(commit_key.bytes()) {
        *destination = source;
    }
    Ok(Measurement {
        commit,
        elapsed_nanoseconds,
    })
}

/// Resolves a Git reference to a commit object without changing the checkout.
fn resolve_commit(repository_root: &Path, git_reference: &str) -> Result<String, String> {
    let revision = format!("{git_reference}^{{commit}}");
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(["rev-parse", "--verify", "--end-of-options"])
        .arg(revision)
        .output()
        .map_err(|error| format!("cannot execute git rev-parse: {error}"))?;
    if !output.status.success() {
        return Err(format_command_failure("git rev-parse", &output));
    }
    let commit = String::from_utf8(output.stdout)
        .map_err(|error| format!("git returned non-UTF-8 commit output: {error}"))?
        .trim()
        .to_string();
    if commit.is_empty() {
        return Err("git returned an empty commit object".into());
    }
    Ok(commit)
}

/// Builds one release CLI in its detached worktree, outside timed launches.
fn build_runtime(repository_root: &Path, commit: &str) -> Result<BuiltRuntime, String> {
    let worktree = TemporaryWorktree::create(repository_root, commit)?;
    let target_directory = worktree.path.join("target");
    let output = Command::new("cargo")
        .args(["build", "--release", "--quiet", "-p", "eck-cli"])
        .current_dir(&worktree.path)
        .env("CARGO_TARGET_DIR", &target_directory)
        .output()
        .map_err(|error| format!("cannot execute release build: {error}"))?;
    if !output.status.success() {
        return Err(format_command_failure(
            "cargo build --release -p eck-cli",
            &output,
        ));
    }
    let binary_path = target_directory
        .join("release")
        .join(format!("eck{}", env::consts::EXE_SUFFIX));
    if !binary_path.is_file() {
        return Err(format!(
            "release build succeeded but {} was not created",
            binary_path.display()
        ));
    }
    Ok(BuiltRuntime {
        worktree,
        binary_path,
    })
}

/// Runs one Eck process and returns wall-clock duration in nanoseconds.
fn run_once(
    binary_path: &Path,
    working_directory: &Path,
    source_path: &Path,
) -> Result<u128, String> {
    let start = Instant::now();
    let output = Command::new(binary_path)
        .arg(source_path)
        .current_dir(working_directory)
        .output()
        .map_err(|error| format!("cannot execute {}: {error}", binary_path.display()))?;
    let elapsed_nanoseconds = start.elapsed().as_nanos();
    if !output.status.success() {
        return Err(format_command_failure(
            &format!("{}", binary_path.display()),
            &output,
        ));
    }
    Ok(elapsed_nanoseconds)
}

/// Formats a checkpoint result and its delta from the preceding row when both measured.
fn render_measurement_row(
    checkpoint: &BenchmarkCheckpoint,
    result: &Result<Measurement, String>,
    previous_measurement: Option<Measurement>,
) -> String {
    match result {
        Ok(measurement) => {
            let median_milliseconds = measurement.elapsed_nanoseconds as f64 / 1_000_000.0;
            let delta = previous_measurement
                .and_then(|previous| {
                    percentage_delta(
                        previous.elapsed_nanoseconds,
                        measurement.elapsed_nanoseconds,
                    )
                })
                .map(|value| format!("{value:+.2}%"))
                .unwrap_or_else(|| "—".into());
            format!(
                "{:<24} {:<12} {:>11.3} ms {:>12}",
                checkpoint.name,
                String::from_utf8_lossy(&measurement.commit),
                median_milliseconds,
                delta
            )
        }
        Err(error) => {
            let status = if error.starts_with("unsupported") {
                "UNSUPPORTED"
            } else {
                "ERROR"
            };
            format!(
                "{:<24} {:<12} {:>14} {:>12} {}",
                checkpoint.name,
                "—",
                status,
                "—",
                compact_error(error)
            )
        }
    }
}

/// Calculates the percentage change from one non-zero duration to another.
fn percentage_delta(previous_nanoseconds: u128, current_nanoseconds: u128) -> Option<f64> {
    if previous_nanoseconds == 0 {
        None
    } else {
        Some(
            (current_nanoseconds as f64 - previous_nanoseconds as f64)
                / previous_nanoseconds as f64
                * 100.0,
        )
    }
}

/// Creates a unique path without creating it, for Git worktrees and sources.
fn unique_temporary_path(prefix: &str) -> Result<PathBuf, String> {
    for _ in 0..TEMPORARY_PATH_ATTEMPTS {
        let path = env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            process::id(),
            TEMPORARY_PATH_COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        if !path.exists() {
            return Ok(path);
        }
    }
    Err(format!(
        "cannot allocate a unique temporary path after {TEMPORARY_PATH_ATTEMPTS} attempts"
    ))
}

/// Reserves one unique temporary directory atomically for source materialization.
fn reserve_temporary_directory(prefix: &str) -> Result<PathBuf, String> {
    for _ in 0..TEMPORARY_PATH_ATTEMPTS {
        let path = unique_temporary_path(prefix)?;
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "cannot create temporary directory {}: {error}",
                    path.display()
                ));
            }
        }
    }
    Err(format!(
        "cannot reserve a unique temporary directory after {TEMPORARY_PATH_ATTEMPTS} attempts"
    ))
}

/// Removes only one explicitly identified temporary path.
fn remove_exact_path(path: &Path) {
    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else if path.exists() {
        let _ = fs::remove_file(path);
    }
}

/// Removes one worktree and its owned container, pruning stale Git metadata
/// only when the targeted worktree removal fails.
fn cleanup_worktree(container: &Path, repository_root: &Path, worktree_path: &Path) {
    let removal = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(["worktree", "remove", "--force"])
        .arg(worktree_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let removal_succeeded = removal.is_ok_and(|status| status.success());

    remove_exact_path(worktree_path);
    if !removal_succeeded {
        let _ = Command::new("git")
            .arg("-C")
            .arg(repository_root)
            .args(["worktree", "prune", "--expire", "now"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    remove_exact_path(container);
}

/// Turns a failed child process into a concise readable runner error.
fn format_command_failure(command_name: &str, output: &process::Output) -> String {
    let standard_error = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if standard_error.is_empty() {
        format!("{command_name} failed with status {}", output.status)
    } else {
        format!(
            "{command_name} failed with status {}: {standard_error}",
            output.status
        )
    }
}

/// Collapses and bounds a child-process error for one table row.
fn compact_error(error: &str) -> String {
    let compacted = error.split_whitespace().collect::<Vec<_>>().join(" ");
    if compacted.len() <= MAX_ERROR_DETAIL_LENGTH {
        compacted
    } else {
        let boundary = compacted
            .char_indices()
            .take_while(|(index, _)| *index < MAX_ERROR_DETAIL_LENGTH.saturating_sub(3))
            .map(|(index, character)| index + character.len_utf8())
            .last()
            .unwrap_or(0);
        format!("{}...", &compacted[..boundary])
    }
}
