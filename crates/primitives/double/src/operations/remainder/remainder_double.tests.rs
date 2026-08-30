use super::*;

/// Verifies double remainder and zero-divisor rejection.
#[test]
fn calculates_double_remainder_and_rejects_zero() {
    let left_operand = Value::new(crate::test_type_id(), 10.5_f64);
    let right_operand = Value::new(crate::test_type_id(), 4.0_f64);
    let zero = Value::new(crate::test_type_id(), 0.0_f64);

    let result = remainder_double(&left_operand, &right_operand).unwrap();

    assert_eq!(*result.downcast_ref::<f64>().unwrap(), 2.5);
    assert!(matches!(
        remainder_double(&left_operand, &zero),
        Err(CoreError::DivisionByZero)
    ));
}
