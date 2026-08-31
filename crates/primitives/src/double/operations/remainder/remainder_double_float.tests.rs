use super::*;

/// Verifies ordered double/float remainder and zero-divisor rejection.
#[test]
fn calculates_float_double_remainder_in_both_orders_and_rejects_zero() {
    let float = Value::new(crate::double::test_type_id(), 10.5_f32);
    let double = Value::new(crate::double::test_type_id(), 2.0_f64);
    let zero = Value::new(crate::double::test_type_id(), 0.0_f64);

    let float_left = remainder_double_float(&float, &double).unwrap();
    let double_left = remainder_double_float(&double, &float).unwrap();

    assert_eq!(*float_left.downcast_ref::<f64>().unwrap(), 0.5);
    assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 2.0);
    assert!(matches!(
        remainder_double_float(&float, &zero),
        Err(CoreError::DivisionByZero)
    ));
}
