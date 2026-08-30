use super::*;

/// Verifies ordered double/float division and zero-divisor rejection.
#[test]
fn divides_float_and_double_in_both_orders_and_rejects_zero() {
    let float = Value::new(crate::test_type_id(), 5.0_f32);
    let double = Value::new(crate::test_type_id(), 2.0_f64);
    let zero = Value::new(crate::test_type_id(), 0.0_f64);

    let float_left = division_double_float(&float, &double).unwrap();
    let double_left = division_double_float(&double, &float).unwrap();

    assert_eq!(*float_left.downcast_ref::<f64>().unwrap(), 2.5);
    assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 0.4);
    assert!(matches!(
        division_double_float(&float, &zero),
        Err(CoreError::DivisionByZero)
    ));
}
