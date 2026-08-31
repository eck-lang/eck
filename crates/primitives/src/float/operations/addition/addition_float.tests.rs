use super::*;

/// Verifies addition of two float values.
#[test]
fn adds_float_values() {
    let left_operand = Value::new(crate::float::test_type_id(), 1.5_f32);
    let right_operand = Value::new(crate::float::test_type_id(), 2.25_f32);

    let result = addition_float(&left_operand, &right_operand).unwrap();

    assert_eq!(*result.downcast_ref::<f32>().unwrap(), 3.75);
}
