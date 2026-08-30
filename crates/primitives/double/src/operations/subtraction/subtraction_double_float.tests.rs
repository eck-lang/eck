use super::*;

/// Verifies that double/float subtraction preserves operand order.
#[test]
fn subtracts_float_and_double_in_both_orders() {
    let float = Value::new(crate::test_type_id(), 1.5_f32);
    let double = Value::new(crate::test_type_id(), 2.25_f64);

    let float_left = subtraction_double_float(&float, &double).unwrap();
    let double_left = subtraction_double_float(&double, &float).unwrap();

    assert_eq!(*float_left.downcast_ref::<f64>().unwrap(), -0.75);
    assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 0.75);
}
