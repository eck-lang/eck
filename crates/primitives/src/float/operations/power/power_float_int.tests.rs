use language_core::{Registry, Value};

use super::power_float_int;

/// Allocates distinct integer and float type identifiers for mixed-operation tests.
fn type_ids() -> (language_core::TypeId, language_core::TypeId) {
    let mut registry = Registry::new();
    (registry.allocate_type_id(), registry.allocate_type_id())
}

/// Verifies that float/integer power preserves base and exponent order.
#[test]
fn raises_float_and_integer_to_powers_in_both_orders() {
    let (integer_id, float_id) = type_ids();
    let float = Value::new(float_id, 2.0_f32);
    let integer = Value::new(integer_id, 3_i64);

    let float_base = power_float_int(&float, &integer).unwrap();
    let integer_base = power_float_int(&integer, &float).unwrap();

    assert_eq!(float_base.type_id(), float_id);
    assert_eq!(integer_base.type_id(), float_id);
    assert_eq!(*float_base.downcast_ref::<f32>().unwrap(), 8.0);
    assert_eq!(*integer_base.downcast_ref::<f32>().unwrap(), 9.0);
}
