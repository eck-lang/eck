use super::*;

/// Verifies that float literals use single-precision representation.
#[test]
fn parses_single_precision_literals() {
    let value = parse("16777217", crate::float::test_type_id()).unwrap();

    assert_eq!(*value.downcast_ref::<f32>().unwrap(), 16_777_216.0);
}

/// Verifies that malformed float literals are rejected.
#[test]
fn rejects_invalid_float_literals() {
    assert!(matches!(
        parse("not-a-number", crate::float::test_type_id()),
        Err(CoreError::InvalidLiteral { .. })
    ));
}
