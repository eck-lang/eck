use super::*;
use crate::semantic::{CoreError, Registry};

/// Builds a registry containing every primitive registered by `register_all`.
fn primitive_registry() -> Registry {
    let mut registry = Registry::new();
    register_all(&mut registry).expect("primitive registration must succeed");
    registry
}

/// Verifies the façade registers every built-in primitive type by canonical name.
#[test]
fn registers_every_primitive_type() {
    let registry = primitive_registry();
    for type_name in [
        "bool", "decimal", "double", "float", "string", "null", "regex", "int8", "int16", "int32",
        "int64", "int128", "bigint", "uint8", "uint64",
    ] {
        assert!(
            registry.type_by_name(type_name).is_some(),
            "primitive type `{type_name}` must be registered"
        );
    }
}

/// Verifies the façade installs the aliases that name the default primitives.
#[test]
fn registers_canonical_type_aliases() {
    let registry = primitive_registry();
    assert_eq!(registry.type_by_name("int"), registry.type_by_name("int64"));
    assert_eq!(
        registry.type_by_name("uint"),
        registry.type_by_name("uint64")
    );
}

/// Verifies the façade assigns the default type used by every literal category.
#[test]
fn sets_every_literal_default_type() {
    let registry = primitive_registry();
    for (default, expected_name) in [
        (registry.default_integer(), "int64"),
        (registry.default_fractional(), "decimal"),
        (registry.default_string(), "string"),
        (registry.default_regex(), "regex"),
        (registry.default_boolean(), "bool"),
        (registry.default_null(), "null"),
    ] {
        assert_eq!(
            default.unwrap(),
            registry.type_by_name(expected_name).unwrap(),
            "default for `{expected_name}` must resolve to its registered type"
        );
    }
}

/// Verifies the façade marks exactly the integer primitives with integer capability.
#[test]
fn marks_integer_primitive_capability() {
    let registry = primitive_registry();
    for type_name in [
        "int8", "int16", "int32", "int64", "int128", "bigint", "uint8", "uint64",
    ] {
        let id = registry.type_by_name(type_name).unwrap();
        assert!(
            registry.is_integer_type(id).unwrap(),
            "`{type_name}` must be registered as an integer type"
        );
    }
    for type_name in [
        "decimal", "double", "float", "bool", "string", "null", "regex",
    ] {
        let id = registry.type_by_name(type_name).unwrap();
        assert!(
            !registry.is_integer_type(id).unwrap(),
            "`{type_name}` must not be registered as an integer type"
        );
    }
}

/// Verifies the façade exposes the String namespace bound to the string receiver type.
#[test]
fn exposes_the_string_namespace() {
    let registry = primitive_registry();
    assert!(registry.has_namespace("String"));
    let string_type = registry.type_by_name("string").unwrap();
    assert!(
        registry
            .resolve_receiver_function(string_type, "uppercase", &[string_type])
            .is_ok()
    );
}

/// Verifies registering the façade twice reports a duplicate type instead of overwriting.
#[test]
fn rejects_a_second_registration() {
    let mut registry = primitive_registry();
    assert!(matches!(
        register_all(&mut registry),
        Err(CoreError::DuplicateType(_))
    ));
}
