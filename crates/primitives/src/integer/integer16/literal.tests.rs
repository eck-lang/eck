use super::*;

/// Verifies parsing at the signed 16-bit boundaries.
#[test]
fn parses_signed_16_bit_integer_literals() {
    let minimum = parse("-32768", crate::integer::integer16::test_type_id()).unwrap();
    let maximum = parse("32767", crate::integer::integer16::test_type_id()).unwrap();

    assert_eq!(*minimum.downcast_ref::<i16>().unwrap(), i16::MIN);
    assert_eq!(*maximum.downcast_ref::<i16>().unwrap(), i16::MAX);
}

/// Verifies that invalid and out-of-range source literals are rejected.
#[test]
fn rejects_out_of_range_and_non_numeric_literals() {
    for raw_text in ["32768", "-32769", "not-an-integer"] {
        assert!(matches!(
            parse(raw_text, crate::integer::integer16::test_type_id()),
            Err(CoreError::InvalidLiteral { .. })
        ));
    }
}
