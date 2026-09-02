use language_core::{ExecutionContext, Extension, Registry};

use super::*;

/// Verifies that `remove` deletes all occurrences of the target.
#[test]
fn removes_all_occurrences() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    for (input, target, expected) in [
        ("hello world", "l", "heo word"),
        ("aaa", "a", ""),
        ("prova", "x", "prova"),
        ("", "a", ""),
        ("café café", "é", "caf caf"),
        ("hello   world", " ", "helloworld"),
    ] {
        let receiver = registry.parse_string(input, None).unwrap();
        let target_value = registry.parse_string(target, None).unwrap();
        let result = remove(&context, &[receiver, target_value])
            .unwrap()
            .unwrap();
        assert_eq!(
            result.downcast_ref::<String>().unwrap(),
            expected,
            "remove({input:?}, {target:?})"
        );
    }
}

/// Verifies that `remove` rejects empty targets and invalid arities.
#[test]
fn rejects_invalid_remove_inputs() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();
    crate::IntegerExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let string_value = registry.parse_string("hello", None).unwrap();
    let integer_value = registry.parse_numeric("42", None).unwrap();

    assert!(remove(&context, &[]).is_err());
    assert!(remove(&context, &[string_value.clone()]).is_err());
    assert!(remove(
        &context,
        &[
            string_value.clone(),
            string_value.clone(),
            string_value.clone()
        ]
    )
    .is_err());
    let empty = registry.parse_string("", None).unwrap();
    assert!(remove(&context, &[string_value.clone(), empty]).is_err());
    assert!(remove(&context, &[integer_value, string_value.clone()]).is_err());
}
