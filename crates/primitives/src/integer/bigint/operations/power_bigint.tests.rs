use super::*;
use num_bigint::BigInt;

/// Builds a `bigint` runtime value from a decimal string.
fn bigint_value(raw_text: &str) -> Value {
    Value::new(
        crate::integer::bigint::test_type_id(),
        raw_text.parse::<BigInt>().unwrap(),
    )
}

/// Verifies integer power, exponent validation, and unbounded results.
#[test]
fn raises_integers_to_non_negative_powers_and_rejects_invalid_exponents() {
    let base = bigint_value("2");
    let exponent = bigint_value("130");
    let small_exponent = bigint_value("6");
    let negative = bigint_value("-1");
    let too_large = bigint_value("4294967296");

    let large_result = power_integer(&base, &exponent).unwrap();

    assert_eq!(
        large_result.downcast_ref::<BigInt>().unwrap().to_string(),
        "1361129467683753853853498429727072845824"
    );
    let small_result = power_integer(&base, &small_exponent).unwrap();
    assert_eq!(
        *small_result.downcast_ref::<BigInt>().unwrap(),
        BigInt::from(64)
    );
    assert!(matches!(
        power_integer(&base, &negative),
        Err(CoreError::Runtime(message)) if message.contains("non-negative")
    ));
    assert!(matches!(
        power_integer(&base, &too_large),
        Err(CoreError::Runtime(message)) if message.contains("non-negative")
    ));
}

/// Verifies mixed power promotes both orders and validates the exponent.
#[test]
fn raises_promoted_narrower_operands_as_bigint() {
    let wider_id = crate::integer::bigint::test_type_id();
    let narrower_id = crate::integer::integer8::test_type_id();
    let wide_base = Value::new(wider_id, BigInt::from(2));
    let narrow_exponent = Value::new(narrower_id, 6_i8);
    let narrow_base = Value::new(narrower_id, 2_i8);
    let wide_exponent = Value::new(wider_id, BigInt::from(6));
    let negative_exponent = Value::new(narrower_id, -1_i8);

    for result in [
        power_mixed_integer(&wide_base, &narrow_exponent).unwrap(),
        power_mixed_integer(&narrow_base, &wide_exponent).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), BigInt::from(64));
    }
    assert!(matches!(
        power_mixed_integer(&wide_base, &negative_exponent),
        Err(CoreError::Runtime(message)) if message.contains("non-negative")
    ));
}
