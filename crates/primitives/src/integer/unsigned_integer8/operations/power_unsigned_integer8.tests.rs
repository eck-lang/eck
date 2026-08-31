use super::*;

/// Verifies unsigned integer power and overflow handling.
#[test]
fn powers_unsigned_integers_and_rejects_overflow() {
    let base = Value::new(crate::integer::unsigned_integer8::test_type_id(), 2_u8);
    let exponent = Value::new(crate::integer::unsigned_integer8::test_type_id(), 7_u8);
    let maximum = Value::new(crate::integer::unsigned_integer8::test_type_id(), u8::MAX);
    let two = Value::new(crate::integer::unsigned_integer8::test_type_id(), 2_u8);

    let result = power_unsigned_integer(&base, &exponent).unwrap();

    assert_eq!(*result.downcast_ref::<u8>().unwrap(), 128);
    assert!(matches!(
        power_unsigned_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
