use super::*;

/// Verifies parsing at the unsigned 64-bit boundaries.
#[test]
fn parses_unsigned_64_bit_integer_literals() {
    let minimum = parse("0", crate::integer::unsigned_integer64::test_type_id()).unwrap();
    let maximum = parse(
        "18446744073709551615",
        crate::integer::unsigned_integer64::test_type_id(),
    )
    .unwrap();

    assert_eq!(*minimum.downcast_ref::<u64>().unwrap(), u64::MIN);
    assert_eq!(*maximum.downcast_ref::<u64>().unwrap(), u64::MAX);
}

/// Verifies that invalid and out-of-range source literals are rejected.
#[test]
fn rejects_out_of_range_and_non_numeric_literals() {
    for raw_text in ["18446744073709551616", "-1", "not-an-integer"] {
        assert!(matches!(
            parse(raw_text, crate::integer::unsigned_integer64::test_type_id()),
            Err(CoreError::InvalidLiteral { .. })
        ));
    }
}
