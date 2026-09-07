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
            eck_binary: Some("eck".into()),
            search_roots: vec!["testing/use-cases".into()],
        })
    );
}

/// Selects a focused run for search roots alone, deferring to the default binary.
#[test]
fn selects_focused_run_without_a_binary_flag() {
    let mode = select_mode(os_arguments(&["testing/use-cases"])).unwrap();

    assert_eq!(
        mode,
        Mode::Focused(RunnerArguments {
            eck_binary: None,
            search_roots: vec!["testing/use-cases".into()],
        })
    );
}
