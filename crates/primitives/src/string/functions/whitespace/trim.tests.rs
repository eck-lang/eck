use language_core::{ExecutionContext, Extension, Registry};

use super::*;

/// Verifies that `trim` removes Unicode whitespace from both ends.
#[test]
fn trims_string_on_both_ends() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    for (input, expected) in [
        ("  hello  ", "hello"),
        ("\t\nhello\r\n", "hello"),
        ("hello", "hello"),
        ("", ""),
        ("   ", ""),
        ("  a  b  ", "a  b"),
    ] {
        let receiver = registry.parse_string(input, None).unwrap();
        let result = trim(&context, &[receiver]).unwrap().unwrap();
        assert_eq!(result.downcast_ref::<String>().unwrap(), expected);
    }
}

/// Verifies that `trim` rejects invalid arities and non-string receivers.
#[test]
fn rejects_invalid_trim_inputs() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    crate::IntegerExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let string_value = registry.parse_string("hello", None).unwrap();
    let integer_value = registry.parse_numeric("42", None).unwrap();

    assert!(trim(&context, &[]).is_err());
    assert!(trim(&context, &[string_value.clone(), string_value.clone()]).is_err());
    assert!(trim(&context, &[integer_value]).is_err());
}
