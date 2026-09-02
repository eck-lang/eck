use language_core::{ExecutionContext, Extension, Registry};

use super::*;

/// Verifies that `normalize_space` trims and collapses consecutive whitespace.
#[test]
fn normalizes_consecutive_whitespace() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    for (input, expected) in [
        ("  hello   world  ", "hello world"),
        ("\t\nhello\r\n  world\t", "hello world"),
        ("hello", "hello"),
        ("", ""),
        ("   ", ""),
        ("  a  b  c  ", "a b c"),
        ("a    b", "a b"),
        (" a\tb\nc ", "a b c"),
    ] {
        let receiver = registry.parse_string(input, None).unwrap();
        let result = normalize_space(&context, &[receiver]).unwrap().unwrap();
        assert_eq!(
            result.downcast_ref::<String>().unwrap(),
            expected,
            "normalize_space({input:?})"
        );
    }
}

/// Verifies that `normalize_space` rejects invalid arities and non-string receivers.
#[test]
fn rejects_invalid_normalize_space_inputs() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    crate::IntegerExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let string_value = registry.parse_string("hello", None).unwrap();
    let integer_value = registry.parse_numeric("42", None).unwrap();

    assert!(normalize_space(&context, &[]).is_err());
    assert!(normalize_space(&context, &[string_value.clone(), string_value.clone()]).is_err());
    assert!(normalize_space(&context, &[integer_value]).is_err());
}
