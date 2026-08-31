use super::*;

/// Verifies double/float multiplication in both operand orders.
#[test]
fn multiplies_float_and_double_in_both_orders() {
    let float = Value::new(crate::double::test_type_id(), 1.5_f32);
    let double = Value::new(crate::double::test_type_id(), 4.0_f64);

    let float_left = multiplication_double_float(&float, &double).unwrap();
    let double_left = multiplication_double_float(&double, &float).unwrap();

    assert_eq!(*float_left.downcast_ref::<f64>().unwrap(), 6.0);
    assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 6.0);
}
