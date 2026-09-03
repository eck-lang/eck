use language_core::{ExecutionContext, Extension, Registry};

use super::*;

/// Verifies that `replace` with regex replaces according to global flag.
#[test]
fn replaces_with_regex() {
    let mut registry = Registry::new();
    crate::IntegerExtension.register(&mut registry).unwrap();
    crate::StringExtension.register(&mut registry).unwrap();
    crate::RegexExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    // Global replaces all.
    let receiver = registry.parse_string("aaa", None).unwrap();
    let pattern = registry.parse_regex("/a/g", None).unwrap();
    let replacement = registry.parse_string("b", None).unwrap();
    let result = replace_regex(&context, &[receiver, pattern, replacement])
        .unwrap()
        .unwrap();
    assert_eq!(result.downcast_ref::<String>().unwrap(), "bbb");

    // Without global replaces only first.
    let receiver = registry.parse_string("aaa", None).unwrap();
    let pattern = registry.parse_regex("/a/", None).unwrap();
    let replacement = registry.parse_string("b", None).unwrap();
    let result = replace_regex(&context, &[receiver, pattern, replacement])
        .unwrap()
        .unwrap();
    assert_eq!(result.downcast_ref::<String>().unwrap(), "baa");

    // Case-insensitive flag.
    let receiver = registry.parse_string("Hello HELLO", None).unwrap();
    let pattern = registry.parse_regex("/hello/gi", None).unwrap();
    let replacement = registry.parse_string("hi", None).unwrap();
    let result = replace_regex(&context, &[receiver, pattern, replacement])
        .unwrap()
        .unwrap();
    assert_eq!(result.downcast_ref::<String>().unwrap(), "hi hi");

    // Capture groups.
    let receiver = registry.parse_string("hello world", None).unwrap();
    let pattern = registry.parse_regex(r"/(\w+) (\w+)/", None).unwrap();
    let replacement = registry.parse_string("$2 $1", None).unwrap();
    let result = replace_regex(&context, &[receiver, pattern, replacement])
        .unwrap()
        .unwrap();
    assert_eq!(result.downcast_ref::<String>().unwrap(), "world hello");
}

/// Verifies that regex replace rejects invalid inputs.
#[test]
fn rejects_invalid_regex_replace_inputs() {
    let mut registry = Registry::new();
    crate::IntegerExtension.register(&mut registry).unwrap();
    crate::StringExtension.register(&mut registry).unwrap();
    crate::RegexExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let string_value = registry.parse_string("hello", None).unwrap();
    let regex_value = registry.parse_regex("/l/g", None).unwrap();
    let integer_value = registry.parse_numeric("42", None).unwrap();

    assert!(replace_regex(&context, &[]).is_err());
    assert!(replace_regex(&context, &[string_value.clone(), regex_value.clone()]).is_err());
    assert!(replace_regex(
        &context,
        &[
            string_value.clone(),
            regex_value.clone(),
            string_value.clone(),
            string_value.clone()
        ]
    )
    .is_err());
    assert!(
        replace_regex(
            &context,
            &[
                string_value.clone(),
                string_value.clone(),
                string_value.clone()
            ]
        )
        .is_err()
    );
    assert!(
        replace_regex(
            &context,
            &[integer_value.clone(), regex_value.clone(), string_value.clone()]
        )
        .is_err()
    );
}
