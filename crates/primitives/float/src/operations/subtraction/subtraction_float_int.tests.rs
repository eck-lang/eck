use language_core::{Registry, Value};

use super::subtraction_float_int;

/// Allocates distinct integer and float type identifiers for mixed-operation tests.
fn type_ids() -> (language_core::TypeId, language_core::TypeId) {
    let mut registry = Registry::new();
    (registry.allocate_type_id(), registry.allocate_type_id())
}

/// Verifies that float/integer subtraction preserves operand order.
#[test]
fn subtracts_float_and_integer_in_both_orders() {
    let (integer_id, float_id) = type_ids();
    let float = Value::new(float_id, 1.5_f32);
    let integer = Value::new(integer_id, 4_i64);

    let float_left = subtraction_float_int(&float, &integer).unwrap();
    let integer_left = subtraction_float_int(&integer, &float).unwrap();

    assert_eq!(float_left.type_id(), float_id);
    assert_eq!(integer_left.type_id(), float_id);
    assert_eq!(*float_left.downcast_ref::<f32>().unwrap(), -2.5);
    assert_eq!(*integer_left.downcast_ref::<f32>().unwrap(), 2.5);
}
