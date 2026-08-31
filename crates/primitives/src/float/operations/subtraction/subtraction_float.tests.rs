use super::*;

/// Verifies subtraction of two float values.
#[test]
fn subtracts_float_values() {
    let left_operand = Value::new(crate::float::test_type_id(), 5.5_f32);
    let right_operand = Value::new(crate::float::test_type_id(), 2.25_f32);

    let result = subtraction_float(&left_operand, &right_operand).unwrap();

    assert_eq!(*result.downcast_ref::<f32>().unwrap(), 3.25);
}
