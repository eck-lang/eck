use language_core::{Registry, Value};

use super::multiplication_double_int;

/// Allocates distinct integer and double type identifiers for mixed-operation tests.
fn type_ids() -> (language_core::TypeId, language_core::TypeId) {
    let mut registry = Registry::new();
    (registry.allocate_type_id(), registry.allocate_type_id())
}

/// Verifies double/integer multiplication in both operand orders.
#[test]
fn multiplies_double_and_integer_in_both_orders() {
    let (integer_id, double_id) = type_ids();
    let double = Value::new(double_id, 1.5_f64);
    let integer = Value::new(integer_id, 4_i64);

    let double_left = multiplication_double_int(&double, &integer).unwrap();
    let integer_left = multiplication_double_int(&integer, &double).unwrap();

    assert_eq!(double_left.type_id(), double_id);
    assert_eq!(integer_left.type_id(), double_id);
    assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 6.0);
    assert_eq!(*integer_left.downcast_ref::<f64>().unwrap(), 6.0);
}
