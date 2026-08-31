use super::*;

/// Verifies unsigned integer addition and checked overflow handling.
#[test]
fn adds_unsigned_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::unsigned_integer64::test_type_id(), 15_u64);
    let rhs = Value::new(crate::integer::unsigned_integer64::test_type_id(), 27_u64);
    let maximum = Value::new(crate::integer::unsigned_integer64::test_type_id(), u64::MAX);
    let one = Value::new(crate::integer::unsigned_integer64::test_type_id(), 1_u64);

    let result = addition_unsigned_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<u64>().unwrap(), 42);
    assert!(matches!(
        addition_unsigned_integer(&maximum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
