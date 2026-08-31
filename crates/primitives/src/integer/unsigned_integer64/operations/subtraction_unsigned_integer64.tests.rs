use super::*;

/// Verifies unsigned integer subtraction and underflow handling.
#[test]
fn subtracts_unsigned_integers_and_rejects_underflow() {
    let lhs = Value::new(crate::integer::unsigned_integer64::test_type_id(), 27_u64);
    let rhs = Value::new(crate::integer::unsigned_integer64::test_type_id(), 15_u64);
    let zero = Value::new(crate::integer::unsigned_integer64::test_type_id(), 0_u64);
    let one = Value::new(crate::integer::unsigned_integer64::test_type_id(), 1_u64);

    let result = subtraction_unsigned_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<u64>().unwrap(), 12);
    assert!(matches!(
        subtraction_unsigned_integer(&zero, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
