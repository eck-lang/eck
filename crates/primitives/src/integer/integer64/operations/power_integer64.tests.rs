use super::*;

/// Verifies integer power, exponent validation, and checked overflow handling.
#[test]
fn raises_integers_to_non_negative_powers_and_rejects_invalid_exponents() {
    let base = Value::new(crate::integer::integer64::test_type_id(), 2_i64);
    let exponent = Value::new(crate::integer::integer64::test_type_id(), 10_i64);
    let negative = Value::new(crate::integer::integer64::test_type_id(), -1_i64);
    let maximum = Value::new(crate::integer::integer64::test_type_id(), i64::MAX);
    let two = Value::new(crate::integer::integer64::test_type_id(), 2_i64);

    let result = power_integer(&base, &exponent).unwrap();

    assert_eq!(*result.downcast_ref::<i64>().unwrap(), 1024);
    assert!(matches!(
        power_integer(&base, &negative),
        Err(CoreError::Runtime(message)) if message.contains("non-negative")
    ));
    assert!(matches!(
        power_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
