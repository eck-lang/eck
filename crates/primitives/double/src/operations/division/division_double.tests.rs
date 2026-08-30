use super::*;

/// Verifies double division and zero-divisor rejection.
#[test]
fn divides_double_values_and_rejects_zero() {
    let left_operand = Value::new(crate::test_type_id(), 9.0_f64);
    let right_operand = Value::new(crate::test_type_id(), 4.0_f64);
    let zero = Value::new(crate::test_type_id(), 0.0_f64);

    let result = division_double(&left_operand, &right_operand).unwrap();

    assert_eq!(*result.downcast_ref::<f64>().unwrap(), 2.25);
    assert!(matches!(
        division_double(&left_operand, &zero),
        Err(CoreError::DivisionByZero)
    ));
}
