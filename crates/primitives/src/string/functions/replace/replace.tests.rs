use language_core::{ExecutionContext, Extension, Registry};

use super::*;

/// Verifies that `replace` substitutes all occurrences of the target.
#[test]
fn replaces_all_occurrences() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    for (input, target, replacement, expected) in [
        ("hello world", "world", "ECK", "hello ECK"),
        ("aaa", "a", "b", "bbb"),
        ("prova", "x", "y", "prova"),
        ("", "a", "b", ""),
        ("café café", "é", "e", "cafe cafe"),
    ] {
        let receiver = registry.parse_string(input, None).unwrap();
        let target_value = registry.parse_string(target, None).unwrap();
        let replacement_value = registry.parse_string(replacement, None).unwrap();
        let result = replace(&context, &[receiver, target_value, replacement_value])
            .unwrap()
            .unwrap();
        assert_eq!(result.downcast_ref::<String>().unwrap(), expected);
    }
}

/// Verifies that `replace` rejects empty targets and invalid arities.
#[test]
fn rejects_invalid_replace_inputs() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    crate::IntegerExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let string_value = registry.parse_string("hello", None).unwrap();
    let integer_value = registry.parse_numeric("42", None).unwrap();

    assert!(replace(&context, &[]).is_err());
    assert!(replace(&context, &[string_value.clone(), string_value.clone()]).is_err());
    assert!(
        replace(
            &context,
            &[
                string_value.clone(),
                string_value.clone(),
                string_value.clone(),
                string_value.clone()
            ]
        )
        .is_err()
    );
    let empty = registry.parse_string("", None).unwrap();
    assert!(
        replace(
            &context,
            &[string_value.clone(), empty, string_value.clone()]
        )
        .is_err()
    );
    assert!(
        replace(
            &context,
            &[integer_value, string_value.clone(), string_value]
        )
        .is_err()
    );
}
