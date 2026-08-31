use super::*;

/// Verifies float division and zero-divisor rejection.
#[test]
fn divides_float_values_and_rejects_zero() {
    let left_operand = Value::new(crate::float::test_type_id(), 9.0_f32);
    let right_operand = Value::new(crate::float::test_type_id(), 4.0_f32);
    let zero = Value::new(crate::float::test_type_id(), 0.0_f32);

    let result = division_float(&left_operand, &right_operand).unwrap();

    assert_eq!(*result.downcast_ref::<f32>().unwrap(), 2.25);
    assert!(matches!(
        division_float(&left_operand, &zero),
        Err(CoreError::DivisionByZero)
    ));
}
