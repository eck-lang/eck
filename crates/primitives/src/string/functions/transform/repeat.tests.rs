use language_core::{ExecutionContext, Extension, Registry};

use super::*;

/// Verifies that `repeat` repeats strings the requested number of times.
#[test]
fn repeats_string_the_requested_times() {
    let mut registry = Registry::new();
    crate::IntegerExtension.register(&mut registry).unwrap();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    for (input, count, expected) in [
        ("ab", 3, "ababab"),
        ("prova", 1, "prova"),
        ("prova", 0, ""),
        ("", 5, ""),
        ("café", 2, "cafécafé"),
        ("😀", 3, "😀😀😀"),
        ("hi", 5, "hihihihihi"),
    ] {
        let receiver = registry.parse_string(input, None).unwrap();
        let count_value = registry.parse_numeric(&count.to_string(), None).unwrap();
        let result = repeat(&context, &[receiver, count_value])
            .unwrap()
            .unwrap();
        assert_eq!(
            result.downcast_ref::<String>().unwrap(),
            expected,
            "repeat({input:?}, {count})"
        );
    }
}

/// Verifies that `repeat` rejects negative counts and invalid arities.
#[test]
fn rejects_invalid_repeat_inputs() {
    let mut registry = Registry::new();
    crate::IntegerExtension.register(&mut registry).unwrap();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let string_value = registry.parse_string("hello", None).unwrap();
    let integer_value = registry.parse_numeric("2", None).unwrap();

    assert!(repeat(&context, &[]).is_err());
    assert!(repeat(&context, &[string_value.clone()]).is_err());
    assert!(repeat(
        &context,
        &[
            string_value.clone(),
            integer_value.clone(),
            integer_value.clone()
        ]
    )
    .is_err());
    assert!(repeat(&context, &[string_value.clone(), string_value.clone()]).is_err());
    assert!(repeat(&context, &[integer_value.clone(), integer_value]).is_err());

    let negative = registry.parse_numeric("-1", None).unwrap();
    assert!(repeat(&context, &[string_value.clone(), negative]).is_err());
}
