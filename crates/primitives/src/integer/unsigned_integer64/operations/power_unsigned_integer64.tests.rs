use super::*;

/// Verifies unsigned integer power and overflow handling.
#[test]
fn powers_unsigned_integers_and_rejects_overflow() {
    let base = Value::new(crate::integer::unsigned_integer64::test_type_id(), 2_u64);
    let exponent = Value::new(crate::integer::unsigned_integer64::test_type_id(), 10_u64);
    let maximum = Value::new(crate::integer::unsigned_integer64::test_type_id(), u64::MAX);
    let two = Value::new(crate::integer::unsigned_integer64::test_type_id(), 2_u64);
    let large_exponent = Value::new(
        crate::integer::unsigned_integer64::test_type_id(),
        u64::from(u32::MAX) + 1,
    );

    let result = power_unsigned_integer(&base, &exponent).unwrap();

    assert_eq!(*result.downcast_ref::<u64>().unwrap(), 1024);
    assert!(matches!(
        power_unsigned_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
    assert!(matches!(
        power_unsigned_integer(&base, &large_exponent),
        Err(CoreError::Runtime(message)) if message.contains("fit in u32")
    ));
}
