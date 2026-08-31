use super::*;

/// Verifies integer power, exponent validation, and checked overflow handling.
#[test]
fn raises_integers_to_non_negative_powers_and_rejects_invalid_exponents() {
    let base = Value::new(crate::integer::integer8::test_type_id(), 2_i8);
    let exponent = Value::new(crate::integer::integer8::test_type_id(), 6_i8);
    let negative = Value::new(crate::integer::integer8::test_type_id(), -1_i8);
    let maximum = Value::new(crate::integer::integer8::test_type_id(), i8::MAX);
    let two = Value::new(crate::integer::integer8::test_type_id(), 2_i8);

    let result = power_integer(&base, &exponent).unwrap();

    assert_eq!(*result.downcast_ref::<i8>().unwrap(), 64);
    assert!(matches!(
        power_integer(&base, &negative),
        Err(CoreError::Runtime(message)) if message.contains("non-negative")
    ));
    assert!(matches!(
        power_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
