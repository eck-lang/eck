use super::*;

/// Verifies multiplication of two double values.
#[test]
fn multiplies_double_values() {
    let left_operand = Value::new(crate::double::test_type_id(), 1.5_f64);
    let right_operand = Value::new(crate::double::test_type_id(), 4.0_f64);

    let result = multiplication_double(&left_operand, &right_operand).unwrap();

    assert_eq!(*result.downcast_ref::<f64>().unwrap(), 6.0);
}
