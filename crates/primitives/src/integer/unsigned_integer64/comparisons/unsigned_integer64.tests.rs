use super::*;

/// Verifies all ordering outcomes for ordinary unsigned integer values.
#[test]
fn compares_unsigned_integer_payloads() {
    let smaller = Value::new(crate::integer::unsigned_integer64::test_type_id(), 4_u64);
    let equal_value = Value::new(crate::integer::unsigned_integer64::test_type_id(), 4_u64);
    let larger = Value::new(crate::integer::unsigned_integer64::test_type_id(), 7_u64);

    assert!(equal(&smaller, &equal_value).unwrap());
    assert!(!not_equal(&smaller, &equal_value).unwrap());
    assert!(less(&smaller, &larger).unwrap());
    assert!(less_or_equal(&smaller, &equal_value).unwrap());
    assert!(greater(&larger, &smaller).unwrap());
    assert!(greater_or_equal(&larger, &smaller).unwrap());
}

/// Verifies total ordering at the unsigned 64-bit boundaries.
#[test]
fn compares_unsigned_integer_boundaries() {
    let minimum = Value::new(crate::integer::unsigned_integer64::test_type_id(), u64::MIN);
    let maximum = Value::new(crate::integer::unsigned_integer64::test_type_id(), u64::MAX);

    assert!(less(&minimum, &maximum).unwrap());
    assert!(greater(&maximum, &minimum).unwrap());
}

/// Verifies that comparison rejects a non-unsigned-integer runtime representation.
#[test]
fn rejects_non_unsigned_integer_payloads() {
    let unsigned_integer = Value::new(crate::integer::unsigned_integer64::test_type_id(), 1_u64);
    let float = Value::new(crate::integer::unsigned_integer64::test_type_id(), 1.0_f32);

    assert!(matches!(
        equal(&unsigned_integer, &float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "uint64"
    ));
}
