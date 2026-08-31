use super::*;

/// Verifies integer remainder, zero-divisor rejection, and checked overflow.
#[test]
fn calculates_integer_remainder_and_rejects_zero_and_overflow() {
    let lhs = Value::new(crate::integer::integer8::test_type_id(), 43_i8);
    let rhs = Value::new(crate::integer::integer8::test_type_id(), 5_i8);
    let zero = Value::new(crate::integer::integer8::test_type_id(), 0_i8);
    let minimum = Value::new(crate::integer::integer8::test_type_id(), i8::MIN);
    let negative_one = Value::new(crate::integer::integer8::test_type_id(), -1_i8);

    let result = remainder_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i8>().unwrap(), 3);
    assert!(matches!(
        remainder_integer(&lhs, &zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        remainder_integer(&minimum, &negative_one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
