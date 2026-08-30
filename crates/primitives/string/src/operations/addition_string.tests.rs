use language_core::Registry;

use super::*;

/// Verifies ordered string concatenation, including empty operands.
#[test]
fn concatenates_string_payloads() {
    let mut registry = Registry::new();
    let string_type = registry.allocate_type_id();
    let left = Value::new(string_type, "hello".to_owned());
    let right = Value::new(string_type, " world".to_owned());
    let empty = Value::new(string_type, String::new());

    let result = addition_string(&left, &right).unwrap();
    let unchanged = addition_string(&left, &empty).unwrap();

    assert_eq!(result.downcast_ref::<String>().unwrap(), "hello world");
    assert_eq!(unchanged.downcast_ref::<String>().unwrap(), "hello");
}
