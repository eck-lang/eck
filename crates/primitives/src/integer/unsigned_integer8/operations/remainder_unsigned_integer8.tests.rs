use super::*;

/// Verifies unsigned integer remainder and zero divisor handling.
#[test]
fn remainder_unsigned_integers_and_rejects_zero_divisor() {
    let lhs = Value::new(crate::integer::unsigned_integer8::test_type_id(), 42_u8);
    let rhs = Value::new(crate::integer::unsigned_integer8::test_type_id(), 5_u8);
    let zero = Value::new(crate::integer::unsigned_integer8::test_type_id(), 0_u8);

    let result = remainder_unsigned_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<u8>().unwrap(), 2);
    assert!(matches!(
        remainder_unsigned_integer(&lhs, &zero),
        Err(CoreError::DivisionByZero)
    ));
}
