use language_core::{ExecutionContext, Extension, Registry};

use super::*;

/// Verifies that `uppercase` converts ASCII and Unicode text to uppercase.
#[test]
fn converts_string_to_uppercase() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    for (input, expected) in [
        ("prova", "PROVA"),
        ("", ""),
        ("Hello World", "HELLO WORLD"),
        ("àèìòù", "ÀÈÌÒÙ"),
        ("ß", "SS"),
    ] {
        let receiver = registry.parse_string(input, None).unwrap();
        let result = uppercase(&context, &[receiver]).unwrap().unwrap();
        assert_eq!(result.downcast_ref::<String>().unwrap(), expected);
    }
}

/// Verifies that `uppercase` rejects invalid arities and non-string receivers.
#[test]
fn rejects_invalid_uppercase_inputs() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    crate::IntegerExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let string_value = registry.parse_string("hello", None).unwrap();
    let integer_value = registry.parse_numeric("42", None).unwrap();

    assert!(uppercase(&context, &[]).is_err());
    assert!(uppercase(&context, &[string_value.clone(), string_value.clone()]).is_err());
    assert!(uppercase(&context, &[integer_value]).is_err());
}
