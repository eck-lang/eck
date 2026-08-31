use super::*;

/// Verifies boolean equality and inequality for both possible values.
#[test]
fn compares_boolean_payloads_for_equality_only() {
    let true_value = Value::new(crate::boolean::test_type_id(), true);
    let equal_true_value = Value::new(crate::boolean::test_type_id(), true);
    let false_value = Value::new(crate::boolean::test_type_id(), false);

    assert!(equal(&true_value, &equal_true_value).unwrap());
    assert!(!not_equal(&true_value, &equal_true_value).unwrap());
    assert!(!equal(&true_value, &false_value).unwrap());
    assert!(not_equal(&true_value, &false_value).unwrap());
}

/// Verifies that comparison rejects a non-boolean runtime representation.
#[test]
fn rejects_non_boolean_payloads() {
    let boolean = Value::new(crate::boolean::test_type_id(), true);
    let integer = Value::new(crate::boolean::test_type_id(), 1_i64);

    assert!(matches!(
        equal(&boolean, &integer),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "bool"
    ));
}
