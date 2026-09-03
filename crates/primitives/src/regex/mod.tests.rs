use language_core::{Extension, Registry};

/// Verifies that the regex type is registered and is the default.
#[test]
fn registers_regex_type() {
    let mut registry = Registry::new();
    crate::RegexExtension.register(&mut registry).unwrap();

    assert!(registry.type_by_name("regex").is_some());
    assert!(registry.default_regex().is_ok());
}
