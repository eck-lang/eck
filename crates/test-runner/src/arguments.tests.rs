use super::*;

/// Parses an explicit binary with several search roots.
#[test]
fn parses_binary_and_search_roots() {
    let arguments = parse_arguments(
        [
            "--binary",
            "eck",
            "testing/use-cases",
            "testing/regressions",
        ]
        .into_iter()
        .map(std::ffi::OsString::from),
    )
    .unwrap();

    assert_eq!(
        arguments,
        RunnerArguments {
            eck_binary: "eck".into(),
            search_roots: vec!["testing/use-cases".into(), "testing/regressions".into()],
        }
    );
}

/// Resolves a relative binary path against the current directory.
#[test]
fn resolves_relative_binary_against_current_directory() {
    let arguments = parse_arguments(
        ["--binary", "target/debug/eck", "testing/use-cases"]
            .into_iter()
            .map(std::ffi::OsString::from),
    )
    .unwrap();

    assert_eq!(
        arguments.eck_binary,
        std::env::current_dir().unwrap().join("target/debug/eck")
    );
}

/// Rejects argument lists that name no binary flag.
#[test]
fn rejects_missing_binary_flag() {
    let error = parse_arguments(
        ["testing/use-cases"]
            .into_iter()
            .map(std::ffi::OsString::from),
    )
    .unwrap_err();

    assert!(error.starts_with("usage:"));
}

/// Rejects a binary flag without a binary path.
#[test]
fn rejects_missing_binary_path() {
    let error =
        parse_arguments(["--binary"].into_iter().map(std::ffi::OsString::from)).unwrap_err();

    assert!(error.starts_with("usage:"));
}

/// Rejects a binary without search roots.
#[test]
fn rejects_missing_search_roots() {
    let error = parse_arguments(
        ["--binary", "eck"]
            .into_iter()
            .map(std::ffi::OsString::from),
    )
    .unwrap_err();

    assert!(error.starts_with("usage:"));
}
