use super::*;

/// Verifies integer division, zero-divisor rejection, and checked overflow.
#[test]
fn divides_integers_and_rejects_zero_and_overflow() {
    let lhs = Value::new(crate::integer::integer8::test_type_id(), 43_i8);
    let rhs = Value::new(crate::integer::integer8::test_type_id(), 5_i8);
    let zero = Value::new(crate::integer::integer8::test_type_id(), 0_i8);
    let minimum = Value::new(crate::integer::integer8::test_type_id(), i8::MIN);
    let negative_one = Value::new(crate::integer::integer8::test_type_id(), -1_i8);

    let result = division_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i8>().unwrap(), 8);
    assert!(matches!(
        division_integer(&lhs, &zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        division_integer(&minimum, &negative_one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
