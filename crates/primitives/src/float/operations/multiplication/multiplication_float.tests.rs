use super::*;

/// Verifies multiplication of two float values.
#[test]
fn multiplies_float_values() {
    let left_operand = Value::new(crate::float::test_type_id(), 1.5_f32);
    let right_operand = Value::new(crate::float::test_type_id(), 4.0_f32);

    let result = multiplication_float(&left_operand, &right_operand).unwrap();

    assert_eq!(*result.downcast_ref::<f32>().unwrap(), 6.0);
}
