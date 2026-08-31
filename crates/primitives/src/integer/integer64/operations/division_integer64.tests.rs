use super::*;

/// Verifies integer division, zero-divisor rejection, and checked overflow.
#[test]
fn divides_integers_and_rejects_zero_and_overflow() {
    let lhs = Value::new(crate::integer::integer64::test_type_id(), 43_i64);
    let rhs = Value::new(crate::integer::integer64::test_type_id(), 5_i64);
    let zero = Value::new(crate::integer::integer64::test_type_id(), 0_i64);
    let minimum = Value::new(crate::integer::integer64::test_type_id(), i64::MIN);
    let negative_one = Value::new(crate::integer::integer64::test_type_id(), -1_i64);

    let result = division_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i64>().unwrap(), 8);
    assert!(matches!(
        division_integer(&lhs, &zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        division_integer(&minimum, &negative_one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
