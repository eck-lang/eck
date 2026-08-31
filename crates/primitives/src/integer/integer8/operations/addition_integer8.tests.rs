use super::*;

/// Verifies integer addition and checked overflow handling.
#[test]
fn adds_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer8::test_type_id(), 15_i8);
    let rhs = Value::new(crate::integer::integer8::test_type_id(), 27_i8);
    let maximum = Value::new(crate::integer::integer8::test_type_id(), i8::MAX);
    let one = Value::new(crate::integer::integer8::test_type_id(), 1_i8);

    let result = addition_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i8>().unwrap(), 42);
    assert!(matches!(
        addition_integer(&maximum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
