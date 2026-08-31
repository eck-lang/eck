use language_core::{CoreError, Registry, Value};

use super::remainder_float_int;

/// Allocates distinct integer and float type identifiers for mixed-operation tests.
fn type_ids() -> (language_core::TypeId, language_core::TypeId) {
    let mut registry = Registry::new();
    (registry.allocate_type_id(), registry.allocate_type_id())
}

/// Verifies ordered float/integer remainder and zero-divisor rejection.
#[test]
fn calculates_float_integer_remainder_in_both_orders_and_rejects_zero() {
    let (integer_id, float_id) = type_ids();
    let float = Value::new(float_id, 10.5_f32);
    let integer = Value::new(integer_id, 4_i64);
    let integer_zero = Value::new(integer_id, 0_i64);
    let float_zero = Value::new(float_id, 0.0_f32);

    let float_left = remainder_float_int(&float, &integer).unwrap();
    let integer_left = remainder_float_int(&integer, &float).unwrap();

    assert_eq!(float_left.type_id(), float_id);
    assert_eq!(integer_left.type_id(), float_id);
    assert_eq!(*float_left.downcast_ref::<f32>().unwrap(), 2.5);
    assert_eq!(*integer_left.downcast_ref::<f32>().unwrap(), 4.0);
    assert!(matches!(
        remainder_float_int(&float, &integer_zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        remainder_float_int(&integer, &float_zero),
        Err(CoreError::DivisionByZero)
    ));
}
