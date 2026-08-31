use super::*;

/// Verifies parsing at the unsigned 8-bit boundaries.
#[test]
fn parses_unsigned_8_bit_integer_literals() {
    let minimum = parse("0", crate::integer::unsigned_integer8::test_type_id()).unwrap();
    let maximum = parse("255", crate::integer::unsigned_integer8::test_type_id()).unwrap();

    assert_eq!(*minimum.downcast_ref::<u8>().unwrap(), u8::MIN);
    assert_eq!(*maximum.downcast_ref::<u8>().unwrap(), u8::MAX);
}

/// Verifies that invalid and out-of-range source literals are rejected.
#[test]
fn rejects_out_of_range_and_non_numeric_literals() {
    for raw_text in ["256", "-1", "not-an-integer"] {
        assert!(matches!(
            parse(raw_text, crate::integer::unsigned_integer8::test_type_id()),
            Err(CoreError::InvalidLiteral { .. })
        ));
    }
}
