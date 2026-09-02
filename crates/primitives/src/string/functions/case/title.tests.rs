use language_core::{ExecutionContext, Extension, Registry};

use super::*;

/// Verifies that `title` converts each word's first alphabetic character to uppercase.
#[test]
fn converts_string_to_title_case() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    for (input, expected) in [
        ("hello world", "Hello World"),
        ("HELLO WORLD", "Hello World"),
        ("", ""),
        ("prova", "Prova"),
        ("hello   world", "Hello   World"),
        ("hello-world", "Hello-World"),
        ("they're here", "They'Re Here"),
        ("àèìòù test", "Àèìòù Test"),
        ("123abc foo", "123Abc Foo"),
        ("  leading and trailing  ", "  Leading And Trailing  "),
    ] {
        let receiver = registry.parse_string(input, None).unwrap();
        let result = title(&context, &[receiver]).unwrap().unwrap();
        assert_eq!(
            result.downcast_ref::<String>().unwrap(),
            expected,
            "title({input:?})"
        );
    }
}

/// Verifies that `title` rejects invalid arities and non-string receivers.
#[test]
fn rejects_invalid_title_inputs() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    crate::IntegerExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let string_value = registry.parse_string("hello", None).unwrap();
    let integer_value = registry.parse_numeric("42", None).unwrap();

    assert!(title(&context, &[]).is_err());
    assert!(title(&context, &[string_value.clone(), string_value.clone()]).is_err());
    assert!(title(&context, &[integer_value]).is_err());
}
