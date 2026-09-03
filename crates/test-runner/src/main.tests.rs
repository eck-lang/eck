use super::*;

fn os_arguments(arguments: &[&str]) -> impl Iterator<Item = std::ffi::OsString> {
    arguments
        .iter()
        .map(std::ffi::OsString::from)
        .collect::<Vec<_>>()
        .into_iter()
}

/// Selects the complete suite when no arguments are given.
#[test]
fn selects_complete_suite_without_arguments() {
    assert_eq!(select_mode(Vec::new().into_iter()), Ok(Mode::Complete));
}

/// Selects a focused run for an explicit binary and search roots.
#[test]
fn selects_focused_run_with_binary_and_roots() {
    let mode = select_mode(os_arguments(&["--binary", "eck", "testing/use-cases"])).unwrap();

    assert_eq!(
        mode,
        Mode::Focused(RunnerArguments {
            eck_binary: "eck".into(),
            search_roots: vec!["testing/use-cases".into()],
        })
    );
}

/// Rejects argument lists that name no binary.
#[test]
fn rejects_argument_lists_without_a_binary_flag() {
    let error = select_mode(os_arguments(&["testing/use-cases"])).unwrap_err();

    assert!(error.starts_with("usage:"));
}
