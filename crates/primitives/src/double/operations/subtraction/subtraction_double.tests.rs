use super::*;

/// Verifies subtraction of two double values.
#[test]
fn subtracts_double_values() {
    let left_operand = Value::new(crate::double::test_type_id(), 5.5_f64);
    let right_operand = Value::new(crate::double::test_type_id(), 2.25_f64);

    let result = subtraction_double(&left_operand, &right_operand).unwrap();

    assert_eq!(*result.downcast_ref::<f64>().unwrap(), 3.25);
}
