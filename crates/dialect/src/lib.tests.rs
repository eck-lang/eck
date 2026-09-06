use super::*;

#[test]
fn registry_contains_expected_builtins() {
    let registry = default_registry().expect("registry must build");
    // Core numeric/string/bool types must be present regardless of future additions.
    for name in [
        "int", "float", "double", "decimal", "string", "bool", "null",
    ] {
        assert!(
            registry.type_by_name(name).is_some(),
            "type `{name}` must be registered"
        );
    }
    // Keywords are non-empty and match lexer truth.
    assert!(ECK_KEYWORDS.contains(&"type"));
    assert!(ECK_KEYWORDS.contains(&"@config"));
}

#[test]
fn registry_enumeration_is_exhaustive_and_sorted() {
    let registry = default_registry().unwrap();
    let names: Vec<&str> = registry.registered_type_names().collect();
    assert!(names.contains(&"bool"));
    // Aliases (if any) are included; `boolean` may be added via extension alias.
    // Ensure at least core types are present.
    assert!(names.len() >= 7);
    // Sorted deterministic order for completion stability.
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
}
