use super::*;

use std::ffi::OsStr;

/// Checks the percentage delta used by consecutive benchmark rows.
#[test]
fn calculates_percentage_delta() {
    assert_eq!(percentage_delta(100, 125), Some(25.0));
    assert_eq!(percentage_delta(100, 75), Some(-25.0));
    assert_eq!(percentage_delta(0, 75), None);
}

/// Keeps the first row without a delta and formats measured milliseconds.
#[test]
fn renders_first_measurement_without_delta() {
    let checkpoint = BenchmarkCheckpoint {
        name: "initial".into(),
        git_reference: "HEAD".into(),
        annotation: None,
        comment: None,
    };
    let measurement = Measurement {
        commit: *b"1234567",
        elapsed_nanoseconds: 125_000_000,
    };

    let row = render_measurement_row(&checkpoint, &Ok(measurement), None);

    assert!(row.contains("initial"));
    assert!(row.contains("1234567"));
    assert!(row.contains("125.000 ms"));
    assert!(row.contains("—"));
}

/// Omits a delta after an unsuccessful preceding checkpoint.
#[test]
fn renders_error_without_time_or_delta() {
    let checkpoint = BenchmarkCheckpoint {
        name: "broken".into(),
        git_reference: "missing".into(),
        annotation: None,
        comment: None,
    };

    let row = render_measurement_row(
        &checkpoint,
        &Err("unsupported commit missing: no such ref".into()),
        None,
    );

    assert!(row.contains("UNSUPPORTED"));
    assert!(row.contains("—"));
    assert!(!row.contains("ms"));
    assert!(!row.contains('\n'));
}

/// Collapses multiline process diagnostics to one bounded table row.
#[test]
fn compacts_long_error_details() {
    let detail =
        "first line\nsecond line\twith a very long diagnostic that exceeds the table width "
            .repeat(5);
    let compacted = compact_error(&detail);

    assert!(!compacted.contains('\n'));
    assert!(compacted.contains("first line second line"));
    assert!(compacted.len() <= MAX_ERROR_DETAIL_LENGTH);
    assert!(compacted.ends_with("..."));
}

/// Writes exactly one reusable source file for all runtime invocations.
#[test]
fn temporary_source_preserves_source_text_and_cleans_up() {
    let source_text = "value: int = 1\nprint(value)\n";
    let source = TemporarySource::create(source_text).unwrap();
    assert_eq!(fs::read_to_string(&source.path).unwrap(), source_text);
    let directory = source.directory.clone();
    drop(source);
    assert!(!directory.exists());
}

/// Retries source-directory allocation when the first candidate already exists.
#[test]
fn reserves_source_directory_after_collision() {
    let first_candidate = unique_temporary_path("eck-benchmark-source-collision").unwrap();
    fs::create_dir(&first_candidate).unwrap();

    let reserved = reserve_temporary_directory("eck-benchmark-source-collision").unwrap();
    assert_ne!(reserved, first_candidate);

    remove_exact_path(&first_candidate);
    remove_exact_path(&reserved);
}

/// Creates and removes a detached worktree without changing the repository.
#[test]
fn temporary_worktree_preserves_repository_status() {
    let repository = scratch_git_repository();
    let before = git_output(&repository, ["status", "--porcelain"]);
    let commit = resolve_commit(&repository, "HEAD").unwrap();
    let worktree_path;
    let container_path;
    {
        let worktree = TemporaryWorktree::create(&repository, &commit).unwrap();
        worktree_path = worktree.path.clone();
        container_path = worktree.container.clone();
        assert!(worktree.container.is_dir());
        assert!(worktree_path.join("tracked.txt").is_file());
        assert_eq!(git_output(&repository, ["status", "--porcelain"]), before);
    }
    assert!(!worktree_path.exists());
    assert!(!container_path.exists());
    assert_eq!(git_output(&repository, ["status", "--porcelain"]), before);
    remove_exact_path(&repository);
}

/// Cleans Git metadata when adding a worktree fails before it is usable.
#[test]
fn failed_worktree_add_does_not_leave_stale_metadata() {
    let repository = scratch_git_repository();
    let before = git_output(&repository, ["worktree", "list", "--porcelain"]);

    let error = match TemporaryWorktree::create(&repository, "missing-benchmark-commit") {
        Ok(worktree) => {
            drop(worktree);
            panic!("an unknown commit unexpectedly created a worktree")
        }
        Err(error) => error,
    };

    assert!(error.contains("git worktree add"));
    assert_eq!(
        git_output(&repository, ["worktree", "list", "--porcelain"]),
        before
    );
    remove_exact_path(&repository);
}

/// Creates a tiny local Git repository suitable for worktree lifecycle tests.
fn scratch_git_repository() -> PathBuf {
    let repository = unique_temporary_path("eck-benchmark-git-test").unwrap();
    fs::create_dir(&repository).unwrap();
    git_status(&repository, ["init", "--quiet"]);
    fs::write(repository.join("tracked.txt"), "tracked\n").unwrap();
    git_status(&repository, ["add", "tracked.txt"]);
    git_status_with_options(
        &repository,
        [
            "-c",
            "user.name=Eck Benchmark Test",
            "-c",
            "user.email=eck-benchmark@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "initial benchmark test commit",
        ],
    );
    repository
}

/// Runs a successful Git command in a scratch repository.
fn git_status<const N: usize>(repository: &Path, arguments: [&str; N]) {
    git_status_with_options(repository, arguments);
}

/// Runs a Git command and panics with its output when it fails.
fn git_status_with_options<I, S>(repository: &Path, arguments: I)
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Returns standard output from a successful Git command.
fn git_output<const N: usize>(repository: &Path, arguments: [&str; N]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
