use language_core::{ExecutionContext, Extension, Registry};

use super::*;

/// Verifies that `lowercase` converts ASCII and Unicode text to lowercase.
#[test]
fn converts_string_to_lowercase() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    for (input, expected) in [
        ("PROVA", "prova"),
        ("", ""),
        ("Hello World", "hello world"),
        ("ÀÈÌÒÙ", "àèìòù"),
        ("ECK LANG", "eck lang"),
    ] {
        let receiver = registry.parse_string(input, None).unwrap();
        let result = lowercase(&context, &[receiver]).unwrap().unwrap();
        assert_eq!(result.downcast_ref::<String>().unwrap(), expected);
    }
}

/// Verifies that `lowercase` rejects invalid arities and non-string receivers.
#[test]
fn rejects_invalid_lowercase_inputs() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    crate::IntegerExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let string_value = registry.parse_string("hello", None).unwrap();
    let integer_value = registry.parse_numeric("42", None).unwrap();

    assert!(lowercase(&context, &[]).is_err());
    assert!(lowercase(&context, &[string_value.clone(), string_value.clone()]).is_err());
    assert!(lowercase(&context, &[integer_value]).is_err());
}
