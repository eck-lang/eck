use super::*;

use language_core::{Extension, Registry};

/// Verifies that `get` extracts regex payloads and rejects other types.
#[test]
fn extracts_regex_and_rejects_other_payloads() {
    let mut registry = Registry::new();
    crate::RegexExtension.register(&mut registry).unwrap();
    crate::StringExtension.register(&mut registry).unwrap();

    let regex_value = registry.parse_regex("/abc/g", None).unwrap();
    assert!(get(&regex_value).is_ok());

    let string_value = registry.parse_string("hello", None).unwrap();
    assert!(get(&string_value).is_err());
}
