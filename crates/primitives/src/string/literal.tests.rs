use language_core::Registry;

use super::*;

/// Verifies that literal parsing preserves empty and Unicode text.
#[test]
fn parses_decoded_unicode_text() {
    let mut registry = Registry::new();
    let string_type = registry.allocate_type_id();

    for text in ["", "hello", "L'acqua 😀", "first\nsecond"] {
        let value = parse(text, string_type).unwrap();
        assert_eq!(value.downcast_ref::<String>().unwrap(), text);
    }
}
