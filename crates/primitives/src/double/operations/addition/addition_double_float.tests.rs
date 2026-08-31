use super::*;

/// Verifies double/float addition in both operand orders.
#[test]
fn adds_float_and_double_in_both_orders() {
    let float = Value::new(crate::double::test_type_id(), 1.5_f32);
    let double = Value::new(crate::double::test_type_id(), 2.25_f64);

    let float_left = addition_double_float(&float, &double).unwrap();
    let double_left = addition_double_float(&double, &float).unwrap();

    assert_eq!(float_left.type_id(), double.type_id());
    assert_eq!(*float_left.downcast_ref::<f64>().unwrap(), 3.75);
    assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 3.75);
}
