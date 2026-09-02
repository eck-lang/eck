use language_core::{ExecutionContext, Extension, Registry};

use super::*;

/// Verifies that `pad_end` pads on the end to the target length.
#[test]
fn pads_string_on_end() {
    let mut registry = Registry::new();
    crate::IntegerExtension.register(&mut registry).unwrap();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    let test_cases = [
        ("foo", 5, " ", "foo  "),
        ("foo", 5, "ab", "fooab"),
        ("foo", 6, "ab", "fooaba"),
        ("foo", 2, " ", "foo"),
        ("foo", 3, " ", "foo"),
        ("", 3, "x", "xxx"),
        ("café", 6, "-", "café--"),
        ("hi", 5, "123", "hi123"),
    ];

    for (input, target, pad, expected) in test_cases {
        let receiver = registry.parse_string(input, None).unwrap();
        let target_value = registry.parse_numeric(&target.to_string(), None).unwrap();
        let pad_value = registry.parse_string(pad, None).unwrap();
        let result = pad_end(&context, &[receiver, target_value, pad_value])
            .unwrap()
            .unwrap();
        assert_eq!(
            result.downcast_ref::<String>().unwrap(),
            expected,
            "pad_end({input:?}, {target}, {pad:?})"
        );
    }
}

/// Verifies that `pad_end` rejects invalid inputs.
#[test]
fn rejects_invalid_pad_end_inputs() {
    let mut registry = Registry::new();
    crate::IntegerExtension.register(&mut registry).unwrap();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let string_value = registry.parse_string("hello", None).unwrap();
    let short_string = registry.parse_string("hi", None).unwrap();
    let integer_value = registry.parse_numeric("5", None).unwrap();
    let larger_target = registry.parse_numeric("10", None).unwrap();
    let empty_pad = registry.parse_string("", None).unwrap();

    assert!(pad_end(&context, &[]).is_err());
    assert!(pad_end(&context, &[string_value.clone(), integer_value.clone()]).is_err());
    assert!(pad_end(
        &context,
        &[
            string_value.clone(),
            string_value.clone(),
            string_value.clone()
        ]
    )
    .is_err());
    assert!(pad_end(
        &context,
        &[short_string.clone(), larger_target.clone(), empty_pad.clone()]
    )
    .is_err());

    let negative = registry.parse_numeric("-1", None).unwrap();
    let pad = registry.parse_string(" ", None).unwrap();
    assert!(pad_end(&context, &[string_value.clone(), negative, pad]).is_err());
}
