use super::*;

/// Verifies unsigned integer division and zero divisor handling.
#[test]
fn divides_unsigned_integers_and_rejects_zero_divisor() {
    let lhs = Value::new(crate::integer::unsigned_integer8::test_type_id(), 42_u8);
    let rhs = Value::new(crate::integer::unsigned_integer8::test_type_id(), 6_u8);
    let zero = Value::new(crate::integer::unsigned_integer8::test_type_id(), 0_u8);

    let result = division_unsigned_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<u8>().unwrap(), 7);
    assert!(matches!(
        division_unsigned_integer(&lhs, &zero),
        Err(CoreError::DivisionByZero)
    ));
}
