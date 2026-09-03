use super::*;

use std::sync::atomic::{AtomicUsize, Ordering};

/// Counter suffixing every scratch directory so parallel tests never share one.
static SCRATCH_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Creates an isolated scratch directory below the system temporary directory.
///
/// Callers own the returned path and must remove it when the test completes.
/// The counter suffix keeps parallel tests isolated from each other.
fn scratch_directory(name: &str) -> PathBuf {
    let scratch = std::env::temp_dir().join(format!(
        "eck-test-runner-{}-{name}-{}",
        std::process::id(),
        SCRATCH_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&scratch).unwrap();
    scratch
}

/// Discovers nested `.eckt` files while ignoring other extensions.
#[test]
fn discovers_nested_eckt_files() {
    let root = scratch_directory("nested");
    fs::create_dir_all(root.join("use-cases")).unwrap();
    fs::create_dir_all(root.join("regressions")).unwrap();
    fs::write(root.join("use-cases").join("sums.eckt"), "# Sums").unwrap();
    fs::write(root.join("use-cases").join("notes.txt"), "notes").unwrap();
    fs::write(root.join("regressions").join("fix.eckt"), "# Fix").unwrap();

    let found = discover_test_paths(std::slice::from_ref(&root)).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(
        found,
        vec![
            root.join("regressions").join("fix.eckt"),
            root.join("use-cases").join("sums.eckt"),
        ]
    );
}

/// Deduplicates roots that resolve to the same `.eckt` file.
#[test]
fn deduplicates_overlapping_roots() {
    let root = scratch_directory("overlapping");
    fs::write(root.join("case.eckt"), "# Case").unwrap();

    let found = discover_test_paths(&[root.clone(), root.clone()]).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(found, vec![root.join("case.eckt")]);
}

/// Reports an empty directory as no tests rather than an error.
#[test]
fn reports_empty_directories_as_no_tests() {
    let root = scratch_directory("empty");

    let found = discover_test_paths(std::slice::from_ref(&root)).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert!(found.is_empty());
}

/// Rejects a directly requested file without the `.eckt` extension.
#[test]
fn rejects_non_eckt_files() {
    let root = scratch_directory("rejected");
    let notes = root.join("notes.txt");
    fs::write(&notes, "notes").unwrap();

    let error = discover_test_paths(&[notes]).unwrap_err();
    fs::remove_dir_all(&root).unwrap();

    assert!(error.contains("must use `.eckt`"));
}
