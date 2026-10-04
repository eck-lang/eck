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
    assert!(benchmark.preparation.is_empty());
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

/// Parses preparation lines before ordered checkpoints without consuming checkpoint comments.
#[test]
fn parses_multiline_preparation_before_checkpoints() {
    let preparation = "python generate_data.py\nlinux,macos: sh prepare.sh";
    let benchmark = parse_benchmark(&format!(
        "# Prepared benchmark\n\nGenerates inputs before running.\n\n>>> prepare\n{preparation}\n>>> checkpoint baseline main\nBaseline implementation.\n\n>>> source\nprint(1)\n"
    )).unwrap();
    assert_eq!(
        benchmark.preparation,
        parse_preparation(preparation).unwrap()
    );
    assert_eq!(benchmark.checkpoints.len(), 1);
    assert_eq!(
        benchmark.checkpoints[0].comment.as_deref(),
        Some("Baseline implementation.")
    );
    assert_eq!(benchmark.source, "print(1)\n");
}

/// Parses portable preparation and preserves CRLF source text for HEAD-only workloads.
#[test]
fn parses_head_only_benchmark_with_preparation() {
    let benchmark = parse_benchmark(
        "# Head only\r\n\r\nGenerates inputs.\r\n\r\n>>> prepare\r\npython generate_data.py\r\n>>> source\r\nprint(1)\r\n"
    ).unwrap();
    assert_eq!(benchmark.preparation.len(), 1);
    assert!(benchmark.checkpoints.is_empty());
    assert_eq!(benchmark.source, "print(1)\r\n");
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

/// Rejects duplicate and out-of-order preparation section markers.
#[test]
fn rejects_duplicate_and_out_of_order_preparation() {
    let duplicate_error = parse_benchmark(
        "# Duplicate preparation\n\
         \n\
         Rejects repeated setup sections.\n\
         \n\
         >>> prepare\n\
         python one.py\n\
         >>> prepare\n",
    )
    .unwrap_err();
    assert_eq!(duplicate_error, "section `prepare` occurs more than once");

    let out_of_order_error = parse_benchmark(
        "# Late preparation\n\
         \n\
         Rejects setup after a checkpoint.\n\
         \n\
         >>> checkpoint baseline main\n\
         >>> prepare\n",
    )
    .unwrap_err();
    assert_eq!(
        out_of_order_error,
        "section `prepare` must occur before checkpoints"
    );
}

/// Rejects preparation after source and malformed preparation markers.
#[test]
fn rejects_late_and_malformed_preparation_markers() {
    let late_error = parse_benchmark(
        "# Late preparation\n\
         \n\
         Rejects setup after source.\n\
         \n\
         >>> source\n\
         >>> prepare\n",
    )
    .unwrap_err();
    assert_eq!(
        late_error,
        "section `prepare` must occur before `>>> source`"
    );

    let malformed_error = parse_benchmark(
        "# Malformed preparation\n\
         \n\
         Requires an exact preparation marker.\n\
         \n\
         >>> prepare extra\n",
    )
    .unwrap_err();
    assert_eq!(
        malformed_error,
        "unknown section marker `>>> prepare extra`"
    );
}

/// Propagates preparation grammar errors before executing any workload.
#[test]
fn propagates_preparation_argument_errors() {
    let invalid_preparation = "python \"unclosed.py";
    let expected_error = parse_preparation(invalid_preparation).unwrap_err();
    let benchmark_text = format!(
        "# Invalid preparation\n\nRejects invalid arguments.\n\n>>> prepare\n{invalid_preparation}\n>>> source\nprint(1)\n"
    );
    assert_eq!(
        parse_benchmark(&benchmark_text).unwrap_err(),
        expected_error
    );
}
