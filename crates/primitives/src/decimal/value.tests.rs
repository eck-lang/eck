use super::*;

/// Verifies that a decimal payload can be extracted from a runtime value.
#[test]
fn extracts_decimal_value() {
    let value = Value::new(crate::decimal::test_type_id(7), Decimal::new(1250, 2));

    assert_eq!(get(&value).unwrap(), Decimal::new(1250, 2));
}

/// Verifies that values with another payload type are rejected.
#[test]
fn rejects_non_decimal_value() {
    let value = Value::new(crate::decimal::test_type_id(7), 1250_i64);

    assert!(
        matches!(get(&value), Err(CoreError::InvalidValueRepresentation(name)) if name == "decimal")
    );
}

/// Verifies that NaN and infinity cannot be promoted to decimal values.
#[test]
fn rejects_non_finite_floating_point_promotions() {
    assert!(matches!(from_float(f32::NAN), Err(CoreError::Runtime(_))));
    assert!(matches!(
        from_double(f64::INFINITY),
        Err(CoreError::Runtime(_))
    ));
}
