use super::*;

/// Verifies parsing at the signed 128-bit boundaries.
#[test]
fn parses_signed_128_bit_integer_literals() {
    let minimum = parse(
        "-170141183460469231731687303715884105728",
        crate::integer::integer128::test_type_id(),
    )
    .unwrap();
    let maximum = parse(
        "170141183460469231731687303715884105727",
        crate::integer::integer128::test_type_id(),
    )
    .unwrap();

    assert_eq!(*minimum.downcast_ref::<i128>().unwrap(), i128::MIN);
    assert_eq!(*maximum.downcast_ref::<i128>().unwrap(), i128::MAX);
}

/// Verifies that invalid and out-of-range source literals are rejected.
#[test]
fn rejects_out_of_range_and_non_numeric_literals() {
    for raw_text in [
        "170141183460469231731687303715884105728",
        "-170141183460469231731687303715884105729",
        "not-an-integer",
    ] {
        assert!(matches!(
            parse(raw_text, crate::integer::integer128::test_type_id()),
            Err(CoreError::InvalidLiteral { .. })
        ));
    }
}
