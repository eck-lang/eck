use std::{fs, path::PathBuf, process::Command};

/// Returns a temporary `.eck` path unique to the calling test.
fn temporary_source(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("eck-invocation-{}-{label}.eck", std::process::id()))
}

/// Runs the CLI binary with the given arguments.
fn run(arguments: &[&std::ffi::OsStr]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_eck"))
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn reports_usage_and_exits_nonzero_without_an_input_file() {
    let output = run(&[]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "eck: usage: eck <file.eck>\n"
    );
}

#[test]
fn rejects_paths_without_the_eck_extension_before_reading() {
    let path = temporary_source("wrong-extension").with_extension("txt");
    let output = run(&[path.as_os_str()]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "eck: input file must use the .eck extension\n"
    );
}

#[test]
fn reports_unreadable_input_paths() {
    let path = temporary_source("missing");
    let _ = fs::remove_file(&path);
    let output = run(&[path.as_os_str()]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.starts_with(&format!("eck: cannot read `{}`:", path.display())));
}

#[test]
fn prefixes_compile_errors_and_exits_nonzero() {
    let path = temporary_source("invalid");
    fs::write(&path, "value: int = \"not an int\"\n").unwrap();
    let output = run(&[path.as_os_str()]);
    let _ = fs::remove_file(&path);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("eck: ")
    );
}

#[test]
fn executes_a_valid_program_silently_on_stderr() {
    let path = temporary_source("valid");
    fs::write(&path, "print(21 * 2)\n").unwrap();
    let output = run(&[path.as_os_str()]);
    let _ = fs::remove_file(&path);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "42\n");
    assert!(output.stderr.is_empty());
}
