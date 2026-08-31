use super::*;

/// Verifies unsigned integer subtraction and underflow handling.
#[test]
fn subtracts_unsigned_integers_and_rejects_underflow() {
    let lhs = Value::new(crate::integer::unsigned_integer8::test_type_id(), 27_u8);
    let rhs = Value::new(crate::integer::unsigned_integer8::test_type_id(), 15_u8);
    let zero = Value::new(crate::integer::unsigned_integer8::test_type_id(), 0_u8);
    let one = Value::new(crate::integer::unsigned_integer8::test_type_id(), 1_u8);

    let result = subtraction_unsigned_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<u8>().unwrap(), 12);
    assert!(matches!(
        subtraction_unsigned_integer(&zero, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
