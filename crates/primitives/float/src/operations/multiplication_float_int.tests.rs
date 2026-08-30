use language_core::{Registry, Value};

use super::multiplication_float_int;

fn type_ids() -> (language_core::TypeId, language_core::TypeId) {
    let mut registry = Registry::new();
    let integer = registry.allocate_type_id();
    let float = registry.allocate_type_id();
    (integer, float)
}

#[test]
fn multiplies_float_and_integer_values_in_both_orders() {
    let (integer_id, float_id) = type_ids();
    let float = Value::new(float_id, 1.5_f32);
    let integer = Value::new(integer_id, 4_i64);

    let float_left = multiplication_float_int(&float, &integer).unwrap();
    let integer_left = multiplication_float_int(&integer, &float).unwrap();

    assert_eq!(float_left.type_id(), float_id);
    assert_eq!(integer_left.type_id(), float_id);
    assert_eq!(*float_left.downcast_ref::<f32>().unwrap(), 6.0);
    assert_eq!(*integer_left.downcast_ref::<f32>().unwrap(), 6.0);
}
