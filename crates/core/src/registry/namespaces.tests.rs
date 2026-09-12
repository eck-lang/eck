use super::*;

use crate::FunctionSignature;

use super::super::test_support::{execute_function, register_type};

/// Verifies direct, namespaced, and receiver lookup return one canonical function ID.
#[test]
fn resolves_namespace_and_receiver_calls_to_the_registered_function() {
    let mut registry = Registry::new();
    let string_type = register_type(&mut registry, "string");
    registry
        .register_namespace("String", Some(string_type))
        .unwrap();
    let direct = registry
        .register_function(
            "replace",
            FunctionSignature::Exact(vec![string_type, string_type, string_type]),
            Some(string_type),
            execute_function,
        )
        .unwrap();
    registry
        .export_namespace_function("String", "replace", "replace")
        .unwrap();

    let arguments = [string_type, string_type, string_type];
    assert_eq!(
        registry
            .resolve_namespace_function("String", "replace", &arguments)
            .unwrap(),
        direct
    );
    assert_eq!(
        registry
            .resolve_receiver_function(string_type, "replace", &arguments)
            .unwrap(),
        direct
    );
}

/// Verifies namespace registration rejects duplicate surfaces and invalid exports.
#[test]
fn rejects_invalid_namespace_registrations() {
    let mut registry = Registry::new();
    let string_type = register_type(&mut registry, "string");
    registry
        .register_namespace("String", Some(string_type))
        .unwrap();

    assert!(matches!(
        registry.register_namespace("String", None),
        Err(CoreError::DuplicateNamespace(name)) if name == "String"
    ));
    assert!(matches!(
        registry.export_namespace_function("String", "missing", "missing"),
        Err(CoreError::UnknownFunction(name)) if name == "missing"
    ));

    registry
        .register_function(
            "lowercase",
            FunctionSignature::Exact(vec![string_type]),
            Some(string_type),
            execute_function,
        )
        .unwrap();
    assert!(matches!(
        registry.register_namespace("Text", Some(string_type)),
        Err(CoreError::DuplicateTypeNamespace(name)) if name == "string"
    ));
    assert!(matches!(
        registry.export_namespace_function("Missing", "lowercase", "lowercase"),
        Err(CoreError::UnknownNamespace(name)) if name == "Missing"
    ));
    registry
        .export_namespace_function("String", "lowercase", "lowercase")
        .unwrap();
    assert!(matches!(
        registry.export_namespace_function("String", "lowercase", "lowercase"),
        Err(CoreError::DuplicateNamespaceMember { namespace, member })
            if namespace == "String" && member == "lowercase"
    ));
}

/// Verifies only explicitly global native families are marked globally callable.
#[test]
fn distinguishes_global_functions_from_namespaced_functions() {
    let mut registry = Registry::new();
    let string_type = register_type(&mut registry, "string");
    registry
        .register_function(
            "lowercase",
            FunctionSignature::Exact(vec![string_type]),
            Some(string_type),
            execute_function,
        )
        .unwrap();
    registry
        .register_global_function(
            "string",
            FunctionSignature::AnySingle,
            Some(string_type),
            execute_function,
        )
        .unwrap();

    assert!(!registry.is_global_function("lowercase"));
    assert!(registry.is_global_function("string"));
}
