use super::*;
use std::fs;

/// Structured steps preserve literal arguments and select only their declared hosts.
#[test]
fn parses_portable_and_platform_specific_steps() {
    let steps = parse_preparation(
        r#"python "generate data.py" "two words"
            windows: cmd /d /c prepare.bat"#,
    )
    .unwrap();
    assert!(steps[0].applies_to("windows"));
    assert!(steps[0].applies_to("linux"));
    assert!(steps[0].applies_to("macos"));
    assert!(steps[1].applies_to("windows"));
    assert!(!steps[1].applies_to("linux"));
    assert!(!steps[1].applies_to("macos"));
    let PreparationStep::Python {
        file, arguments, ..
    } = &steps[0]
    else {
        panic!("expected Python step");
    };
    assert_eq!(file, Path::new("generate data.py"));
    assert_eq!(arguments, &["two words"]);
    assert_eq!(python_candidates("windows")[0], ("py", &["-3"][..]));
    assert_eq!(python_candidates("linux")[0], ("python3", &[][..]));
    assert_eq!(python_candidates("macos")[0], ("python3", &[][..]));
}

/// Quoted tokens retain Windows paths, empty arguments and literal shell metacharacters.
#[test]
fn preserves_literal_prepare_arguments() {
    let arguments =
        preparation_arguments(r#"python "C:\data folder\generate.py" '' "a\"b" '$HOME;$(echo x)'"#)
            .unwrap();
    assert_eq!(
        arguments,
        vec![
            "python",
            r"C:\data folder\generate.py",
            "",
            "a\"b",
            "$HOME;$(echo x)"
        ]
    );
}

/// Missing scripts, unsupported hosts and malformed quoting fail before launching a process.
#[test]
fn rejects_invalid_preparation_grammar() {
    for source in [
        "",
        "# comment only",
        "python",
        "python \"\"",
        "\"\"",
        "windows:",
        "unknown: sh prepare.sh",
        "python \"unclosed.py",
    ] {
        assert!(parse_preparation(source).is_err(), "accepted {source}");
    }
}

/// Python and direct commands preserve order and arguments while shared preparation runs once.
#[test]
fn runs_ordered_shared_preparation_from_the_benchmark_directory() {
    let directory = temporary_directory();
    let script = directory.join("record step.py");
    fs::write(
        &script,
        "from pathlib import Path\nimport sys\nwith Path('order.txt').open('a', newline='\\n') as output:\n    output.write(sys.argv[1] + '\\n')\n",
    )
    .unwrap();
    let interpreter = python_command(std::env::consts::OS).unwrap();
    let mut command_arguments = interpreter
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    command_arguments.push(script.to_string_lossy().into_owned());
    command_arguments.push("second step".into());
    let steps = vec![
        PreparationStep::Python {
            file: "record step.py".into(),
            arguments: vec!["first step".into()],
            platforms: vec![],
        },
        PreparationStep::Command {
            program: interpreter.get_program().to_string_lossy().into_owned(),
            arguments: command_arguments,
            platforms: vec![],
        },
    ];
    let mut prepared = HashSet::new();
    prepare_benchmark(&directory, &steps, &mut prepared).unwrap();
    prepare_benchmark(&directory, &steps, &mut prepared).unwrap();
    assert_eq!(
        fs::read_to_string(directory.join("order.txt")).unwrap(),
        "first step\nsecond step\n"
    );
    assert_eq!(prepared.len(), 2);
    let mut alias = steps[0].clone();
    if let PreparationStep::Python { file, .. } = &mut alias {
        *file = "./record step.py".into();
    }
    prepare_benchmark(&directory, &[alias.clone()], &mut prepared).unwrap();
    assert_eq!(prepared.len(), 2);
    if let PreparationStep::Python { arguments, .. } = &mut alias {
        *arguments = vec!["changed argument".into()];
    }
    prepare_benchmark(&directory, &[alias], &mut prepared).unwrap();
    assert_eq!(prepared.len(), 3);
    assert_eq!(
        fs::read_to_string(directory.join("order.txt")).unwrap(),
        "first step\nsecond step\nchanged argument\n"
    );
    fs::remove_dir_all(directory).unwrap();
}

/// A failed Python script runs once and prevents all later steps and timing.
#[test]
fn failed_preparation_stops_without_retrying_the_script() {
    let directory = temporary_directory();
    fs::write(
        directory.join("fail.py"),
        "from pathlib import Path\nimport sys\nwith Path('attempts').open('a') as output:\n    output.write('x')\nsys.exit(7)\n",
    )
    .unwrap();
    fs::write(
        directory.join("later.py"),
        "from pathlib import Path\nPath('later').touch()\n",
    )
    .unwrap();
    let steps = parse_preparation("python fail.py\npython later.py").unwrap();
    let mut prepared = HashSet::new();
    let error = prepare_benchmark(&directory, &steps, &mut prepared).unwrap_err();
    assert!(error.contains("failed"));
    assert_eq!(fs::read_to_string(directory.join("attempts")).unwrap(), "x");
    assert!(!directory.join("later").exists());
    assert!(prepared.is_empty());
    fs::remove_dir_all(directory).unwrap();
}

/// Commands for other platforms are skipped without resolving or executing their program.
#[test]
fn skips_commands_for_other_platforms() {
    let directory = temporary_directory();
    let platform = if std::env::consts::OS == "windows" {
        PreparationPlatform::Linux
    } else {
        PreparationPlatform::Windows
    };
    let steps = vec![PreparationStep::Command {
        program: "missing-program-for-another-platform".into(),
        arguments: vec![],
        platforms: vec![platform],
    }];
    let mut prepared = HashSet::new();
    prepare_benchmark(&directory, &steps, &mut prepared).unwrap();
    assert!(prepared.is_empty());
    fs::remove_dir_all(directory).unwrap();
}

/// Reserves a separate directory for a test's subprocess inputs and outputs.
fn temporary_directory() -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "eck-prepare-test-{}-{timestamp}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    directory
}
