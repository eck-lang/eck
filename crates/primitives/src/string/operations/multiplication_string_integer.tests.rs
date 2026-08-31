use language_core::Registry;

use super::*;

/// Verifies positive and zero string repetition counts.
#[test]
fn repeats_strings_by_non_negative_integer_counts() {
    let mut registry = Registry::new();
    let string_type = registry.allocate_type_id();
    let integer_type = registry.allocate_type_id();
    let text = Value::new(string_type, "ab".to_owned());

    let repeated = multiplication_string_integer(&text, &Value::new(integer_type, 3_i64)).unwrap();
    let empty = multiplication_string_integer(&text, &Value::new(integer_type, 0_i64)).unwrap();

    assert_eq!(repeated.downcast_ref::<String>().unwrap(), "ababab");
    assert_eq!(empty.downcast_ref::<String>().unwrap(), "");
}

/// Verifies rejection of negative counts and invalid integer representations.
#[test]
fn rejects_invalid_string_repetition_counts() {
    let mut registry = Registry::new();
    let string_type = registry.allocate_type_id();
    let integer_type = registry.allocate_type_id();
    let text = Value::new(string_type, "ab".to_owned());

    assert!(matches!(
        multiplication_string_integer(&text, &Value::new(integer_type, -1_i64)),
        Err(CoreError::Runtime(_))
    ));
    assert!(matches!(
        multiplication_string_integer(&text, &Value::new(integer_type, 1_f64)),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
