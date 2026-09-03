use super::*;

/// Reports a mid-text difference with its line and byte offsets.
#[test]
fn reports_mid_text_difference() {
    let difference = format_text_difference("stdout", "first\nsecond\n", "first\nrevised\n");

    assert!(difference.contains("stdout differs at line 2, byte 6"));
}

/// Reports a difference at the very start of the text.
#[test]
fn reports_immediate_difference() {
    let difference = format_text_difference("stderr", "expected\n", "actual\n");

    assert!(difference.contains("stderr differs at line 1, byte 0"));
}

/// Reports a length mismatch at the end of the shorter text.
#[test]
fn reports_length_mismatch() {
    let difference = format_text_difference("stdout", "complete\n", "complete");

    assert!(difference.contains("stdout differs at line 1, byte 8"));
}

/// Creates its isolated directory and removes it on drop.
#[test]
fn removes_temporary_directory_on_drop() {
    let directory = TemporaryCaseDirectory::create(0).unwrap();
    assert!(directory.path.is_dir());

    let path = directory.path.clone();
    drop(directory);

    assert!(!path.exists());
}
