use super::*;

/// Verifies parsing at the signed 32-bit boundaries.
#[test]
fn parses_signed_32_bit_integer_literals() {
    let minimum = parse("-2147483648", crate::integer::integer32::test_type_id()).unwrap();
    let maximum = parse("2147483647", crate::integer::integer32::test_type_id()).unwrap();

    assert_eq!(*minimum.downcast_ref::<i32>().unwrap(), i32::MIN);
    assert_eq!(*maximum.downcast_ref::<i32>().unwrap(), i32::MAX);
}

/// Verifies that invalid and out-of-range source literals are rejected.
#[test]
fn rejects_out_of_range_and_non_numeric_literals() {
    for raw_text in ["2147483648", "-2147483649", "not-an-integer"] {
        assert!(matches!(
            parse(raw_text, crate::integer::integer32::test_type_id()),
            Err(CoreError::InvalidLiteral { .. })
        ));
    }
}
