use language_core::Registry;

use super::*;

/// Verifies extraction of string payloads and rejection of other representations.
#[test]
fn extracts_strings_and_rejects_other_payloads() {
    let mut registry = Registry::new();
    let string_type = registry.allocate_type_id();
    let string = Value::new(string_type, "hello".to_owned());
    let invalid = Value::new(string_type, 1_i64);

    assert_eq!(get(&string).unwrap(), "hello");
    assert!(matches!(
        get(&invalid),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
