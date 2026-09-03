use language_core::{ExecutionContext, Extension, Registry};

/// Verifies that all string pipe functions are registered with the expected signatures.
#[test]
fn registers_string_pipe_functions() {
    let mut registry = Registry::new();
    crate::IntegerExtension.register(&mut registry).unwrap();
    crate::RegexExtension.register(&mut registry).unwrap();
    crate::StringExtension.register(&mut registry).unwrap();
    let string_type = registry.type_by_name("string").unwrap();
    let integer_type = registry.type_by_name("int").unwrap();
    let regex_type = registry.type_by_name("regex").unwrap();

    for name in [
        "uppercase",
        "lowercase",
        "capitalize",
        "title",
        "trim",
        "trim_start",
        "trim_end",
        "normalize_space",
    ] {
        let function = registry
            .resolve_namespace_function("String", name, &[string_type])
            .unwrap_or_else(|_| panic!("{name} should be registered"));
        let descriptor = registry.function(function).unwrap();
        assert_eq!(descriptor.output, Some(string_type));
    }

    let replace_function = registry
        .resolve_namespace_function(
            "String",
            "replace",
            &[string_type, string_type, string_type],
        )
        .unwrap();
    let descriptor = registry.function(replace_function).unwrap();
    assert_eq!(descriptor.output, Some(string_type));

    let remove_function = registry
        .resolve_namespace_function("String", "remove", &[string_type, string_type])
        .unwrap();
    let descriptor = registry.function(remove_function).unwrap();
    assert_eq!(descriptor.output, Some(string_type));

    let pad_start_function = registry
        .resolve_namespace_function(
            "String",
            "pad_start",
            &[string_type, integer_type, string_type],
        )
        .unwrap();
    let descriptor = registry.function(pad_start_function).unwrap();
    assert_eq!(descriptor.output, Some(string_type));

    let pad_end_function = registry
        .resolve_namespace_function(
            "String",
            "pad_end",
            &[string_type, integer_type, string_type],
        )
        .unwrap();
    let descriptor = registry.function(pad_end_function).unwrap();
    assert_eq!(descriptor.output, Some(string_type));

    let repeat_function = registry
        .resolve_namespace_function("String", "repeat", &[string_type, integer_type])
        .unwrap();
    let descriptor = registry.function(repeat_function).unwrap();
    assert_eq!(descriptor.output, Some(string_type));

    let replace_regex_function = registry
        .resolve_namespace_function(
            "String",
            "replace",
            &[string_type, regex_type, string_type],
        )
        .unwrap();
    let descriptor = registry.function(replace_regex_function).unwrap();
    assert_eq!(descriptor.output, Some(string_type));

    let exported_members = registry.namespace_member_names("String").unwrap();
    for name in [
        "uppercase",
        "lowercase",
        "capitalize",
        "title",
        "trim",
        "trim_start",
        "trim_end",
        "normalize_space",
        "replace",
        "remove",
        "pad_start",
        "pad_end",
        "repeat",
    ] {
        assert!(exported_members.contains(&name));
        assert!(!registry.is_global_function(name));
    }
}

/// Verifies that the registered functions can be invoked through the registry.
#[test]
fn executes_registered_pipe_via_registry() {
    let mut registry = Registry::new();
    crate::IntegerExtension.register(&mut registry).unwrap();
    crate::RegexExtension.register(&mut registry).unwrap();
    crate::StringExtension.register(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);

    let string_type = registry.type_by_name("string").unwrap();
    let integer_type = registry.type_by_name("int").unwrap();
    let regex_type = registry.type_by_name("regex").unwrap();

    let uppercase = registry
        .resolve_namespace_function("String", "uppercase", &[string_type])
        .unwrap();
    let receiver = registry.parse_string("prova", None).unwrap();
    let result = (registry.function(uppercase).unwrap().execute)(&context, &[receiver])
        .unwrap()
        .unwrap();
    assert_eq!(result.downcast_ref::<String>().unwrap(), "PROVA");

    let capitalize_id = registry
        .resolve_namespace_function("String", "capitalize", &[string_type])
        .unwrap();
    let receiver = registry.parse_string("hello", None).unwrap();
    let result =
        (registry.function(capitalize_id).unwrap().execute)(&context, &[receiver])
            .unwrap()
            .unwrap();
    assert_eq!(result.downcast_ref::<String>().unwrap(), "Hello");

    let pad_start_id = registry
        .resolve_namespace_function(
            "String",
            "pad_start",
            &[string_type, integer_type, string_type],
        )
        .unwrap();
    let receiver = registry.parse_string("foo", None).unwrap();
    let target = registry.parse_numeric("5", None).unwrap();
    let pad = registry.parse_string(" ", None).unwrap();
    let result = (registry.function(pad_start_id).unwrap().execute)(
        &context,
        &[receiver, target, pad],
    )
    .unwrap()
    .unwrap();
    assert_eq!(result.downcast_ref::<String>().unwrap(), "  foo");

    let repeat_id = registry
        .resolve_namespace_function("String", "repeat", &[string_type, integer_type])
        .unwrap();
    let receiver = registry.parse_string("ab", None).unwrap();
    let count = registry.parse_numeric("3", None).unwrap();
    let result = (registry.function(repeat_id).unwrap().execute)(&context, &[receiver, count])
        .unwrap()
        .unwrap();
    assert_eq!(result.downcast_ref::<String>().unwrap(), "ababab");

    let replace_regex_id = registry
        .resolve_namespace_function(
            "String",
            "replace",
            &[string_type, regex_type, string_type],
        )
        .unwrap();
    let receiver = registry.parse_string("hello 123 world", None).unwrap();
    let pattern = registry.parse_regex(r"/\d+/g", None).unwrap();
    let replacement = registry.parse_string("X", None).unwrap();
    let result = (registry.function(replace_regex_id).unwrap().execute)(
        &context,
        &[receiver, pattern, replacement],
    )
    .unwrap()
    .unwrap();
    assert_eq!(result.downcast_ref::<String>().unwrap(), "hello X world");
}

/// Verifies that integer-dependent functions are not registered when the integer type is missing.
#[test]
fn skips_integer_dependent_functions_without_integer_type() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();

    let string_type = registry.type_by_name("string").unwrap();
    let integer_placeholder = string_type;
    assert!(
        registry
            .resolve_namespace_function(
                "String",
                "pad_start",
                &[string_type, integer_placeholder, string_type],
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_namespace_function(
                "String",
                "pad_end",
                &[string_type, integer_placeholder, string_type],
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_namespace_function(
                "String",
                "repeat",
                &[string_type, integer_placeholder],
            )
            .is_err()
    );
}

/// Verifies that regex-dependent functions are not registered when the regex type is missing.
#[test]
fn skips_regex_dependent_functions_without_regex_type() {
    let mut registry = Registry::new();
    crate::StringExtension.register(&mut registry).unwrap();

    let string_type = registry.type_by_name("string").unwrap();
    assert!(
        registry
            .resolve_namespace_function(
                "String",
                "replace",
                &[string_type, string_type, string_type],
            )
            .is_ok()
    );

    // With regex registered, the additional overload appears.
    let mut with_regex = Registry::new();
    crate::RegexExtension.register(&mut with_regex).unwrap();
    crate::StringExtension.register(&mut with_regex).unwrap();
    let regex_type = with_regex.type_by_name("regex").unwrap();
    let string_type2 = with_regex.type_by_name("string").unwrap();
    assert!(with_regex
        .resolve_namespace_function(
            "String",
            "replace",
            &[string_type2, regex_type, string_type2],
        )
        .is_ok());
}
