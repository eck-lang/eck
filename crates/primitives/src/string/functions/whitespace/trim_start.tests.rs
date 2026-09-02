use language_core::{ExecutionContext, Extension, Registry};

use super::*;

/// Verifies that `trim_start` removes whitespace only from the start.
#[test]
fn trims_string_at_start() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    for (input, expected) in [
        ("  hello  ", "hello  "),
        ("  hello", "hello"),
        ("hello  ", "hello  "),
        ("", ""),
    ] {
        let receiver = registry.parse_string(input, None).unwrap();
        let result = trim_start(&context, &[receiver]).unwrap().unwrap();
        assert_eq!(result.downcast_ref::<String>().unwrap(), expected);
    }
}

/// Verifies that `trim_start` rejects invalid arities and non-string receivers.
#[test]
fn rejects_invalid_trim_start_inputs() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    crate::IntegerExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let string_value = registry.parse_string("hello", None).unwrap();
    let integer_value = registry.parse_numeric("42", None).unwrap();

    assert!(trim_start(&context, &[]).is_err());
    assert!(trim_start(
        &context,
        &[string_value.clone(), string_value.clone()]
    )
    .is_err());
    assert!(trim_start(&context, &[integer_value]).is_err());
}
