//! Recursive discovery of `.eckt` test files.

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
    let mut test_paths = BTreeSet::new();
    for search_root in search_roots {
        discover_test_paths_below(search_root, &mut test_paths)?;
    }
    Ok(test_paths.into_iter().collect())
}

/// Adds the `.eckt` files represented by one file or directory to the result set.
fn discover_test_paths_below(
    path: &Path,
    test_paths: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("cannot inspect `{}`: {error}", path.display()))?;

    if metadata.is_file() {
        if path
            .extension()
            .is_some_and(|extension| extension == "eckt")
        {
            test_paths.insert(path.to_path_buf());
            return Ok(());
        }
        return Err(format!("test file `{}` must use `.eckt`", path.display()));
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
            discover_test_paths_below(&entry_path, test_paths)?;
        } else if file_type.is_file()
            && entry_path
                .extension()
                .is_some_and(|extension| extension == "eckt")
        {
            test_paths.insert(entry_path);
        }
    }

    Ok(())
}
