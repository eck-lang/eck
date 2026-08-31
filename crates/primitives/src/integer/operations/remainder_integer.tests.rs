use super::*;

/// Verifies integer remainder, zero-divisor rejection, and checked overflow.
#[test]
fn calculates_integer_remainder_and_rejects_zero_and_overflow() {
    let lhs = Value::new(crate::integer::test_type_id(), 43_i64);
    let rhs = Value::new(crate::integer::test_type_id(), 5_i64);
    let zero = Value::new(crate::integer::test_type_id(), 0_i64);
    let minimum = Value::new(crate::integer::test_type_id(), i64::MIN);
    let negative_one = Value::new(crate::integer::test_type_id(), -1_i64);

    let result = remainder_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i64>().unwrap(), 3);
    assert!(matches!(
        remainder_integer(&lhs, &zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        remainder_integer(&minimum, &negative_one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
