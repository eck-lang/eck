use super::*;

/// Verifies integer subtraction and checked overflow handling.
#[test]
fn subtracts_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::test_type_id(), 15_i64);
    let rhs = Value::new(crate::test_type_id(), 27_i64);
    let minimum = Value::new(crate::test_type_id(), i64::MIN);
    let one = Value::new(crate::test_type_id(), 1_i64);

    let result = subtraction_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i64>().unwrap(), -12);
    assert!(matches!(
        subtraction_integer(&minimum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
