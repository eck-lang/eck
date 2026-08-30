use super::*;

/// Verifies that double/float power preserves base and exponent order.
#[test]
fn raises_float_and_double_values_to_a_power_in_both_orders() {
    let float = Value::new(crate::test_type_id(), 2.0_f32);
    let double = Value::new(crate::test_type_id(), 3.0_f64);

    let float_left = power_double_float(&float, &double).unwrap();
    let double_left = power_double_float(&double, &float).unwrap();

    assert_eq!(*float_left.downcast_ref::<f64>().unwrap(), 8.0);
    assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 9.0);
}
