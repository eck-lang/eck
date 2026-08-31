use super::*;

/// Verifies that double literals preserve values beyond float precision.
#[test]
fn parses_double_precision_literals() {
    let value = parse("16777217", crate::double::test_type_id()).unwrap();

    assert_eq!(*value.downcast_ref::<f64>().unwrap(), 16_777_217.0);
}

/// Verifies that malformed double literals are rejected.
#[test]
fn rejects_invalid_double_literals() {
    assert!(matches!(
        parse("not-a-number", crate::double::test_type_id()),
        Err(CoreError::InvalidLiteral { .. })
    ));
}
