use language_core::Registry;

use super::*;

/// Verifies unquoted formatting and representation validation.
#[test]
fn formats_string_contents_without_delimiters() {
    let mut registry = Registry::new();
    let string_type = registry.allocate_type_id();

    assert_eq!(
        format(&Value::new(string_type, "hello\nworld".to_owned())).unwrap(),
        "hello\nworld"
    );
    assert!(format(&Value::new(string_type, 1_i64)).is_err());
}
