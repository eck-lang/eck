use language_core::{CoreError, Registry, Value};

use super::division_double_int;

/// Allocates distinct integer and double type identifiers for mixed-operation tests.
fn type_ids() -> (language_core::TypeId, language_core::TypeId) {
    let mut registry = Registry::new();
    (registry.allocate_type_id(), registry.allocate_type_id())
}

/// Verifies ordered double/integer division and zero-divisor rejection.
#[test]
fn divides_double_and_integer_in_both_orders_and_rejects_zero() {
    let (integer_id, double_id) = type_ids();
    let double = Value::new(double_id, 5.0_f64);
    let integer = Value::new(integer_id, 2_i64);
    let integer_zero = Value::new(integer_id, 0_i64);
    let double_zero = Value::new(double_id, 0.0_f64);

    let double_left = division_double_int(&double, &integer).unwrap();
    let integer_left = division_double_int(&integer, &double).unwrap();

    assert_eq!(double_left.type_id(), double_id);
    assert_eq!(integer_left.type_id(), double_id);
    assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 2.5);
    assert_eq!(*integer_left.downcast_ref::<f64>().unwrap(), 0.4);
    assert!(matches!(
        division_double_int(&double, &integer_zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        division_double_int(&integer, &double_zero),
        Err(CoreError::DivisionByZero)
    ));
}
