use super::*;

/// Verifies parsing at the signed 8-bit boundaries.
#[test]
fn parses_signed_8_bit_integer_literals() {
    let minimum = parse("-128", crate::integer::integer8::test_type_id()).unwrap();
    let maximum = parse("127", crate::integer::integer8::test_type_id()).unwrap();

    assert_eq!(*minimum.downcast_ref::<i8>().unwrap(), i8::MIN);
    assert_eq!(*maximum.downcast_ref::<i8>().unwrap(), i8::MAX);
}

/// Verifies that invalid and out-of-range source literals are rejected.
#[test]
fn rejects_out_of_range_and_non_numeric_literals() {
    for raw_text in ["128", "-129", "not-an-integer"] {
        assert!(matches!(
            parse(raw_text, crate::integer::integer8::test_type_id()),
            Err(CoreError::InvalidLiteral { .. })
        ));
    }
}
