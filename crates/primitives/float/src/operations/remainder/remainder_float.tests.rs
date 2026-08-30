use super::*;

/// Verifies float remainder and zero-divisor rejection.
#[test]
fn calculates_float_remainder_and_rejects_zero() {
    let left_operand = Value::new(crate::test_type_id(), 10.5_f32);
    let right_operand = Value::new(crate::test_type_id(), 4.0_f32);
    let zero = Value::new(crate::test_type_id(), 0.0_f32);

    let result = remainder_float(&left_operand, &right_operand).unwrap();

    assert_eq!(*result.downcast_ref::<f32>().unwrap(), 2.5);
    assert!(matches!(
        remainder_float(&left_operand, &zero),
        Err(CoreError::DivisionByZero)
    ));
}
