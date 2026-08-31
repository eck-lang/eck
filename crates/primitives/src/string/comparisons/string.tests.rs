use language_core::Registry;

use super::*;

/// Verifies equality and deterministic lexicographic ordering of Unicode strings.
#[test]
fn compares_string_payloads() {
    let mut registry = Registry::new();
    let string_type = registry.allocate_type_id();
    let alpha = Value::new(string_type, "alpha".to_owned());
    let beta = Value::new(string_type, "beta".to_owned());
    let same_alpha = Value::new(string_type, "alpha".to_owned());

    assert!(equal(&alpha, &same_alpha).unwrap());
    assert!(not_equal(&alpha, &beta).unwrap());
    assert!(less(&alpha, &beta).unwrap());
    assert!(less_or_equal(&alpha, &same_alpha).unwrap());
    assert!(greater(&beta, &alpha).unwrap());
    assert!(greater_or_equal(&beta, &beta).unwrap());
}
