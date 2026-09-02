use language_core::{ExecutionContext, Extension, Registry};

use super::*;

/// Verifies that `capitalize` uppercases the first character and lowercases the rest.
#[test]
fn converts_string_to_capitalized() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    for (input, expected) in [
        ("prova", "Prova"),
        ("PROVA", "Prova"),
        ("", ""),
        ("hello world", "Hello world"),
        ("hELLO", "Hello"),
        ("àèìòù", "Àèìòù"),
        ("ßabc", "SSabc"),
        ("a", "A"),
    ] {
        let receiver = registry.parse_string(input, None).unwrap();
        let result = capitalize(&context, &[receiver]).unwrap().unwrap();
        assert_eq!(
            result.downcast_ref::<String>().unwrap(),
            expected,
            "capitalize({input:?})"
        );
    }
}

/// Verifies that `capitalize` rejects invalid arities and non-string receivers.
#[test]
fn rejects_invalid_capitalize_inputs() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    crate::IntegerExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let string_value = registry.parse_string("hello", None).unwrap();
    let integer_value = registry.parse_numeric("42", None).unwrap();

    assert!(capitalize(&context, &[]).is_err());
    assert!(capitalize(&context, &[string_value.clone(), string_value.clone()]).is_err());
    assert!(capitalize(&context, &[integer_value]).is_err());
}
