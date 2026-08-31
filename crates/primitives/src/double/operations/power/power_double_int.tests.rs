use language_core::{Registry, Value};

use super::power_double_int;

/// Allocates distinct integer and double type identifiers for mixed-operation tests.
fn type_ids() -> (language_core::TypeId, language_core::TypeId) {
    let mut registry = Registry::new();
    (registry.allocate_type_id(), registry.allocate_type_id())
}

/// Verifies that double/integer power preserves base and exponent order.
#[test]
fn raises_double_and_integer_to_powers_in_both_orders() {
    let (integer_id, double_id) = type_ids();
    let double = Value::new(double_id, 2.0_f64);
    let integer = Value::new(integer_id, 3_i64);

    let double_base = power_double_int(&double, &integer).unwrap();
    let integer_base = power_double_int(&integer, &double).unwrap();

    assert_eq!(double_base.type_id(), double_id);
    assert_eq!(integer_base.type_id(), double_id);
    assert_eq!(*double_base.downcast_ref::<f64>().unwrap(), 8.0);
    assert_eq!(*integer_base.downcast_ref::<f64>().unwrap(), 9.0);
}
