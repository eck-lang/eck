use language_core::{Extension, Registry};

/// Verifies that regex literals are parsed and compiled.
#[test]
fn parses_valid_regex_literals() {
    let mut registry = Registry::new();
    crate::RegexExtension.register(&mut registry).unwrap();

    for raw in ["/abc/", "/abc/g", "/abc/i", "/abc/gim", "/a\\/b/"] {
        let value = registry.parse_regex(raw, None).unwrap();
        let regex = crate::regex::value::get(&value).unwrap();
        assert_eq!(regex.raw(), raw);
    }
}

/// Verifies that invalid regex literals are rejected.
#[test]
fn rejects_invalid_regex_literals() {
    let mut registry = Registry::new();
    crate::RegexExtension.register(&mut registry).unwrap();

    assert!(registry.parse_regex("abc", None).is_err());
    assert!(registry.parse_regex("/[unclosed/", None).is_err());
    assert!(registry.parse_regex("/abc/z", None).is_err());
    assert!(registry.parse_regex("/abc/gg", None).is_err());
}
