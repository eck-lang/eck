use super::*;

use language_core::{CoreError, Value};

/// Verifies null equality and inequality are constant for the singleton value.
#[test]
fn compares_null_payloads_for_equality_only() {
    let null = Value::new(crate::null::test_type_id(), crate::null::value::Null);
    let other_null = Value::new(crate::null::test_type_id(), crate::null::value::Null);

    assert!(equal(&null, &other_null).unwrap());
    assert!(!not_equal(&null, &other_null).unwrap());
}

/// Verifies that comparison rejects a non-null runtime representation.
#[test]
fn rejects_non_null_payloads() {
    let null = Value::new(crate::null::test_type_id(), crate::null::value::Null);
    let integer = Value::new(crate::null::test_type_id(), 1_i64);

    assert!(matches!(
        equal(&null, &integer),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "null"
    ));
    assert!(matches!(
        not_equal(&null, &integer),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "null"
    ));
}
