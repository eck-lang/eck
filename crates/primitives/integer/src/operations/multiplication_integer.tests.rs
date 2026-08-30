use super::*;

/// Verifies integer multiplication and checked overflow handling.
#[test]
fn multiplies_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::test_type_id(), 6_i64);
    let rhs = Value::new(crate::test_type_id(), 7_i64);
    let maximum = Value::new(crate::test_type_id(), i64::MAX);
    let two = Value::new(crate::test_type_id(), 2_i64);

    let result = multiplication_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i64>().unwrap(), 42);
    assert!(matches!(
        multiplication_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
