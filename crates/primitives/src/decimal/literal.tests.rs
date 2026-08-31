use super::*;

/// Verifies that decimal literals preserve their exact source precision.
#[test]
fn parses_decimal_literal_without_float_conversion() {
    let value = parse("12.500", crate::decimal::test_type_id(7)).unwrap();

    assert_eq!(value.type_id(), crate::decimal::test_type_id(7));
    assert_eq!(
        *value.downcast_ref::<Decimal>().unwrap(),
        Decimal::new(12500, 3)
    );
}

/// Verifies that invalid decimal literals return a typed compiler error.
#[test]
fn rejects_invalid_decimal_literal() {
    let error = parse("not-a-decimal", crate::decimal::test_type_id(7))
        .err()
        .unwrap();

    assert!(
        matches!(error, CoreError::InvalidLiteral { raw_text, type_name, .. }
        if raw_text == "not-a-decimal" && type_name == "decimal")
    );
}
