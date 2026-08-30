use language_core::{CoreError, Registry, Value};

use super::division_float_int;

/// Allocates distinct integer and float type identifiers for mixed-operation tests.
fn type_ids() -> (language_core::TypeId, language_core::TypeId) {
    let mut registry = Registry::new();
    (registry.allocate_type_id(), registry.allocate_type_id())
}

/// Verifies ordered float/integer division and zero-divisor rejection.
#[test]
fn divides_float_and_integer_in_both_orders_and_rejects_zero() {
    let (integer_id, float_id) = type_ids();
    let float = Value::new(float_id, 5.0_f32);
    let integer = Value::new(integer_id, 2_i64);
    let integer_zero = Value::new(integer_id, 0_i64);
    let float_zero = Value::new(float_id, 0.0_f32);

    let float_left = division_float_int(&float, &integer).unwrap();
    let integer_left = division_float_int(&integer, &float).unwrap();

    assert_eq!(float_left.type_id(), float_id);
    assert_eq!(integer_left.type_id(), float_id);
    assert_eq!(*float_left.downcast_ref::<f32>().unwrap(), 2.5);
    assert_eq!(*integer_left.downcast_ref::<f32>().unwrap(), 0.4);
    assert!(matches!(
        division_float_int(&float, &integer_zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        division_float_int(&integer, &float_zero),
        Err(CoreError::DivisionByZero)
    ));
}
