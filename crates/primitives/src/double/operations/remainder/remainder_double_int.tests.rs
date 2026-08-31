use language_core::{CoreError, Registry, Value};

use super::remainder_double_int;

/// Allocates distinct integer and double type identifiers for mixed-operation tests.
fn type_ids() -> (language_core::TypeId, language_core::TypeId) {
    let mut registry = Registry::new();
    (registry.allocate_type_id(), registry.allocate_type_id())
}

/// Verifies ordered double/integer remainder and zero-divisor rejection.
#[test]
fn calculates_double_integer_remainder_in_both_orders_and_rejects_zero() {
    let (integer_id, double_id) = type_ids();
    let double = Value::new(double_id, 10.5_f64);
    let integer = Value::new(integer_id, 4_i64);
    let integer_zero = Value::new(integer_id, 0_i64);
    let double_zero = Value::new(double_id, 0.0_f64);

    let double_left = remainder_double_int(&double, &integer).unwrap();
    let integer_left = remainder_double_int(&integer, &double).unwrap();

    assert_eq!(double_left.type_id(), double_id);
    assert_eq!(integer_left.type_id(), double_id);
    assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 2.5);
    assert_eq!(*integer_left.downcast_ref::<f64>().unwrap(), 4.0);
    assert!(matches!(
        remainder_double_int(&double, &integer_zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        remainder_double_int(&integer, &double_zero),
        Err(CoreError::DivisionByZero)
    ));
}
