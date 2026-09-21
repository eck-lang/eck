use super::*;

/// Parses title, description, ordered checkpoints, comments, and source.
#[test]
fn parses_benchmark_with_inline_and_multiline_annotations() {
    let benchmark = parse_benchmark(
        "# Decimal addition performance\n\
         \n\
         Measures repeated decimal additions across runtime changes.\n\
         \n\
         >>> checkpoint initial 8af31c2 first stable implementation\n\
         \n\
         Initial implementation used as the historical baseline.\n\
         \n\
         >>> checkpoint numeric-refactor c491bd1 new compact numeric representation\n\
         \n\
         Introduced the new internal numeric representation.\n\
         The implementation removed intermediate allocations.\n\
         \n\
         >>> source\n\
         value: decimal = 0.0000\n\
         value = value + 1.0000\n",
    )
    .unwrap();

    assert_eq!(benchmark.title, "Decimal addition performance");
    assert_eq!(
        benchmark.description,
        "Measures repeated decimal additions across runtime changes."
    );
    assert_eq!(
        benchmark.checkpoints,
        vec![
            BenchmarkCheckpoint {
                name: "initial".into(),
                git_reference: "8af31c2".into(),
                annotation: Some("first stable implementation".into()),
                comment: Some("Initial implementation used as the historical baseline.".into()),
            },
            BenchmarkCheckpoint {
                name: "numeric-refactor".into(),
                git_reference: "c491bd1".into(),
                annotation: Some("new compact numeric representation".into()),
                comment: Some(
                    "Introduced the new internal numeric representation.\nThe implementation removed intermediate allocations."
                        .into(),
                ),
            },
        ]
    );
    assert_eq!(
        benchmark.source,
        "value: decimal = 0.0000\nvalue = value + 1.0000\n"
    );
}

/// Accepts a checkpoint with no inline annotation or multiline comment.
#[test]
fn parses_checkpoint_without_optional_annotations() {
    let benchmark = parse_benchmark(
        "# Minimal benchmark\n\
         \n\
         Measures one workload.\n\
         \n\
         >>> checkpoint baseline main\n\
         \n\
         >>> source\n\
         print(1)\n",
    )
    .unwrap();

    assert_eq!(benchmark.checkpoints[0].annotation, None);
    assert_eq!(benchmark.checkpoints[0].comment, None);
}

/// Preserves CRLF source text and a source without a trailing newline.
#[test]
fn preserves_crlf_and_source_without_final_newline() {
    let benchmark = parse_benchmark(
        "# CRLF benchmark\r\n\
         \r\n\
         Preserves source bytes.\r\n\
         \r\n\
         >>> checkpoint baseline main\r\n\
         baseline comment\r\n\
         \r\n\
         >>> source\r\n\
         first line\r\n\
         second line",
    )
    .unwrap();

    assert_eq!(benchmark.description, "Preserves source bytes.");
    assert_eq!(
        benchmark.checkpoints[0].comment.as_deref(),
        Some("baseline comment")
    );
    assert_eq!(benchmark.source, "first line\r\nsecond line");
}

/// Parses a benchmark that runs only the current HEAD runtime.
#[test]
fn parses_head_only_benchmark_without_checkpoints() {
    let benchmark = parse_benchmark(
        "# Missing checkpoint\n\
         \n\
         The workload has no historical baseline.\n\
         \n\
         >>> source\n\
         print(1)\n",
    )
    .unwrap();

    assert_eq!(benchmark.title, "Missing checkpoint");
    assert_eq!(
        benchmark.description,
        "The workload has no historical baseline."
    );
    assert!(benchmark.checkpoints.is_empty());
    assert_eq!(benchmark.source, "print(1)\n");
}

/// Rejects malformed checkpoint markers with missing required fields.
#[test]
fn rejects_checkpoint_without_git_reference() {
    let error = parse_benchmark(
        "# Malformed checkpoint\n\
         \n\
         Protects parser validation.\n\
         \n\
         >>> checkpoint baseline\n",
    )
    .unwrap_err();

    assert!(error.contains("requires a Git reference"));
}

/// Rejects unknown markers before the benchmark source.
#[test]
fn rejects_unknown_marker() {
    let error = parse_benchmark(
        "# Unknown marker\n\
         \n\
         Protects strict benchmark structure.\n\
         \n\
         >>> config\n",
    )
    .unwrap_err();

    assert_eq!(error, "unknown section marker `>>> config`");
}

/// Rejects a second source marker after the source has started.
#[test]
fn rejects_duplicate_source() {
    let error = parse_benchmark(
        "# Duplicate source\n\
         \n\
         Protects unique source structure.\n\
         \n\
         >>> checkpoint baseline main\n\
         >>> source\n\
         print(1)\n\
         >>> source\n",
    )
    .unwrap_err();

    assert_eq!(error, "section `source` occurs more than once");
}
