use super::*;

/// Parses a successful test while preserving its multiline description and bodies.
#[test]
fn parses_test_with_multiline_description() {
    let test = parse_language_test(
        "# Decimal division\n\
         \n\
         Verifies configured decimal precision.\n\
         The output must retain four fractional digits.\n\
         \n\
         >>> source\n\
         print(1 / 3)\n\
         \n\
         <<< stdout\n\
         0.3333\n\
         \n\
         <<< stderr\n",
    )
    .unwrap();

    assert_eq!(test.title, "Decimal division");
    assert_eq!(
        test.description,
        "Verifies configured decimal precision.\nThe output must retain four fractional digits."
    );
    assert_eq!(test.source, "print(1 / 3)\n");
    assert_eq!(test.expected_standard_output, "0.3333\n");
    assert_eq!(test.expected_standard_error, "");
    assert_eq!(test.expected_exit_code, 0);
}

/// Parses explicit configuration and a non-zero expected exit code.
#[test]
fn parses_configuration_and_exit_code() {
    let test = parse_language_test(
        "# Invalid condition\n\
         \n\
         Protects the diagnostic produced for a non-boolean condition.\n\
         \n\
         >>> config\n\
         [decimal]\n\
         precision = 4\n\
         \n\
         >>> source\n\
         if (1) {}\n\
         \n\
         <<< stdout\n\
         \n\
         <<< stderr\n\
         eck: condition must be bool\n\
         \n\
         <<< exit\n\
         1\n",
    )
    .unwrap();

    assert_eq!(
        test.configuration.as_deref(),
        Some("[decimal]\nprecision = 4\n")
    );
    assert_eq!(test.expected_standard_output, "");
    assert_eq!(
        test.expected_standard_error,
        "eck: condition must be bool\n"
    );
    assert_eq!(test.expected_exit_code, 1);
}

/// Rejects tests that omit their descriptive prose.
#[test]
fn rejects_missing_description() {
    let error = parse_language_test(
        "# Missing description\n\
         >>> source\n\
         print(1)\n\
         <<< stdout\n\
         1\n\
         <<< stderr\n",
    )
    .unwrap_err();

    assert!(error.contains("description is required"));
}

/// Rejects duplicate sections instead of silently using one occurrence.
#[test]
fn rejects_duplicate_sections() {
    let error = parse_language_test(
        "# Duplicate output\n\
         \n\
         Verifies structural validation.\n\
         \n\
         >>> source\n\
         print(1)\n\
         <<< stdout\n\
         1\n\
         <<< stdout\n\
         2\n\
         <<< stderr\n",
    )
    .unwrap_err();

    assert!(error.contains("section `stdout` occurs more than once"));
}

/// Rejects marker names that the format does not define.
#[test]
fn rejects_unknown_sections() {
    let error = parse_language_test(
        "# Unknown input\n\
         \n\
         Verifies structural validation.\n\
         \n\
         >>> file\n\
         print(1)\n",
    )
    .unwrap_err();

    assert_eq!(error, "unknown section marker `>>> file`");
}
