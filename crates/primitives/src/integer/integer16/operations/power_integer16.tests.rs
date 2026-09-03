use super::*;

/// Verifies integer power, exponent validation, and checked overflow handling.
#[test]
fn raises_integers_to_non_negative_powers_and_rejects_invalid_exponents() {
    let base = Value::new(crate::integer::integer16::test_type_id(), 2_i16);
    let exponent = Value::new(crate::integer::integer16::test_type_id(), 6_i16);
    let negative = Value::new(crate::integer::integer16::test_type_id(), -1_i16);
    let maximum = Value::new(crate::integer::integer16::test_type_id(), i16::MAX);
    let two = Value::new(crate::integer::integer16::test_type_id(), 2_i16);

    let result = power_integer(&base, &exponent).unwrap();

    assert_eq!(*result.downcast_ref::<i16>().unwrap(), 64);
    assert!(matches!(
        power_integer(&base, &negative),
        Err(CoreError::Runtime(message)) if message.contains("non-negative")
    ));
    assert!(matches!(
        power_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed power promotes both orders and preserves exponent and overflow errors.
#[test]
fn powers_promoted_integer8_operands_as_integer16() {
    let wider_id = crate::integer::integer16::test_type_id();
    let narrower_id = crate::integer::integer8::test_type_id();
    let wide_two = Value::new(wider_id, 2_i16);
    let narrow_two = Value::new(narrower_id, 2_i8);
    let wide_three = Value::new(wider_id, 3_i16);
    let narrow_three = Value::new(narrower_id, 3_i8);

    for result in [
        power_mixed_integer(&wide_two, &narrow_three).unwrap(),
        power_mixed_integer(&narrow_two, &wide_three).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<i16>().unwrap(), 8);
    }
    let maximum = Value::new(wider_id, i16::MAX);
    assert!(
        matches!(power_mixed_integer(&maximum, &narrow_two), Err(CoreError::Runtime(message)) if message.contains("overflow"))
    );
    let negative = Value::new(narrower_id, -1_i8);
    assert!(
        matches!(power_mixed_integer(&wide_two, &negative), Err(CoreError::Runtime(message)) if message.contains("non-negative"))
    );
    let invalid = Value::new(narrower_id, false);
    assert!(matches!(
        power_mixed_integer(&invalid, &wide_two),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
