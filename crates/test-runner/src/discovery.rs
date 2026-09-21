//! Recursive discovery of Eck test and benchmark documents.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[cfg(test)]
#[path = "discovery.tests.rs"]
mod tests;

/// Recursively discovers `.eckt` files below all requested roots.
pub(crate) fn discover_test_paths(search_roots: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    discover_paths(search_roots, "eckt", "test")
}

/// Recursively discovers case-sensitive `.eckb` files below all requested roots.
pub(crate) fn discover_benchmark_paths(search_roots: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    discover_paths(search_roots, "eckb", "benchmark")
}

/// Recursively discovers documents with one extension below all requested roots.
///
/// The sorted set preserves deterministic output while accepting overlapping
/// roots without returning the same document twice.
fn discover_paths(
    search_roots: &[PathBuf],
    extension: &str,
    document_kind: &str,
) -> Result<Vec<PathBuf>, String> {
    let mut document_paths = BTreeSet::new();
    for search_root in search_roots {
        discover_paths_below(search_root, extension, document_kind, &mut document_paths)?;
    }
    Ok(document_paths.into_iter().collect())
}

/// Adds documents represented by one file or directory to the result set.
fn discover_paths_below(
    path: &Path,
    extension: &str,
    document_kind: &str,
    document_paths: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("cannot inspect `{}`: {error}", path.display()))?;

    if metadata.is_file() {
        if has_extension(path, extension) {
            document_paths.insert(path.to_path_buf());
            return Ok(());
        }
        return Err(format!(
            "{document_kind} file `{}` must use `.{extension}`",
            path.display()
        ));
    }

    if !metadata.is_dir() {
        return Err(format!(
            "test root `{}` is neither a file nor a directory",
            path.display()
        ));
    }

    let mut entries: Vec<_> = fs::read_dir(path)
        .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?
        .collect::<Result<_, _>>()
        .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("cannot inspect `{}`: {error}", entry_path.display()))?;
        if file_type.is_dir() {
            discover_paths_below(&entry_path, extension, document_kind, document_paths)?;
        } else if file_type.is_file() && has_extension(&entry_path, extension) {
            document_paths.insert(entry_path);
        }
    }

    Ok(())
}

/// Returns whether one path uses the requested case-sensitive file extension.
fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .is_some_and(|candidate| candidate == extension)
}
