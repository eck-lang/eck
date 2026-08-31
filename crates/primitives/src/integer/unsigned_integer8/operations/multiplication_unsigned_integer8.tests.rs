use super::*;

/// Verifies unsigned integer multiplication and checked overflow handling.
#[test]
fn multiplies_unsigned_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::unsigned_integer8::test_type_id(), 6_u8);
    let rhs = Value::new(crate::integer::unsigned_integer8::test_type_id(), 7_u8);
    let maximum = Value::new(crate::integer::unsigned_integer8::test_type_id(), u8::MAX);
    let two = Value::new(crate::integer::unsigned_integer8::test_type_id(), 2_u8);

    let result = multiplication_unsigned_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<u8>().unwrap(), 42);
    assert!(matches!(
        multiplication_unsigned_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
