use super::*;

/// Verifies parsing at the signed 64-bit lower boundary.
#[test]
fn parses_signed_64_bit_integer_literals() {
    let value = parse("-9223372036854775808", crate::integer::test_type_id()).unwrap();

    assert_eq!(*value.downcast_ref::<i64>().unwrap(), i64::MIN);
}

/// Verifies that invalid and out-of-range source literals are rejected.
#[test]
fn rejects_out_of_range_and_non_numeric_literals() {
    for raw_text in ["9223372036854775808", "not-an-integer"] {
        assert!(matches!(
            parse(raw_text, crate::integer::test_type_id()),
            Err(CoreError::InvalidLiteral { .. })
        ));
    }
}
