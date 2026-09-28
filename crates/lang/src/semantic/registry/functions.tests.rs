use super::*;

use super::super::test_support::{execute_function, foreign_type_id, register_type};

/// Binds six parameters in callback order and fills omitted optional slots.
#[test]
fn resolves_named_arguments_and_six_parameter_defaults() {
    let mut registry = Registry::new();
    let integer = register_type(&mut registry, "int");
    let function = registry
        .register_function(
            "read",
            FunctionSignature::Exact(vec![integer; 6]),
            None,
            execute_function,
        )
        .unwrap();
    registry
        .set_function_parameter_names(
            function,
            &["path", "delimiter", "header", "quote", "escape", "encoding"],
        )
        .unwrap();
    registry
        .set_function_parameter_defaults(
            function,
            &[
                None,
                Some(crate::semantic::Value::new(integer, 1i64)),
                Some(crate::semantic::Value::new(integer, 2i64)),
                Some(crate::semantic::Value::new(integer, 3i64)),
                Some(crate::semantic::Value::new(integer, 4i64)),
                Some(crate::semantic::Value::new(integer, 5i64)),
            ],
        )
        .unwrap();

    let (resolved, order) = registry
        .resolve_named_function(
            "read",
            &[Some(integer)],
            &[("encoding", Some(integer)), ("delimiter", Some(integer))],
        )
        .unwrap();
    assert_eq!(resolved, function);
    assert_eq!(order, vec![Some(0), Some(2), None, None, None, Some(1)]);
    assert_eq!(
        registry.resolve_function("read", &[integer]).unwrap(),
        function
    );

    assert!(matches!(registry.resolve_named_function("read", &[],
        &[("encoding", Some(integer))]), Err(CoreError::MissingArgument(name)) if name == "path"));
    assert!(
        matches!(registry.resolve_named_function("read", &[Some(integer)],
        &[("path", Some(integer))]), Err(CoreError::DuplicateArgument(name)) if name == "path")
    );
    assert!(
        matches!(registry.resolve_named_function("read", &[Some(integer)],
        &[("unknown", Some(integer))]), Err(CoreError::UnknownNamedArgument(name)) if name == "unknown")
    );
}

#[test]
fn function_types_must_be_registered_before_use() {
    let mut registry = Registry::new();
    let registered = register_type(&mut registry, "int");
    let unknown = foreign_type_id();

    assert!(matches!(
        registry.register_function(
            "identity",
            FunctionSignature::Exact(vec![registered, unknown]),
            None,
            execute_function,
        ),
        Err(CoreError::UnknownTypeId(id)) if id == unknown
    ));
    assert!(matches!(
        registry.register_function(
            "make_unknown",
            FunctionSignature::Exact(vec![registered]),
            Some(unknown),
            execute_function,
        ),
        Err(CoreError::UnknownTypeId(id)) if id == unknown
    ));
}

#[test]
fn function_descriptor_ids_are_scoped_to_their_registry() {
    let registry = Registry::new();
    let mut foreign_registry = Registry::new();
    let foreign_type_id = register_type(&mut foreign_registry, "int");
    let foreign_function = foreign_registry
        .register_function(
            "identity",
            FunctionSignature::Exact(vec![foreign_type_id]),
            Some(foreign_type_id),
            execute_function,
        )
        .unwrap();

    assert!(matches!(
        registry.function(foreign_function),
        Err(CoreError::UnknownFunctionId(id)) if id == foreign_function
    ));
}

#[test]
fn function_overloads_are_deterministic_and_unique() {
    for register_fallback_first in [true, false] {
        let mut registry = Registry::new();
        let type_id = register_type(&mut registry, "int");
        let exact_signature = FunctionSignature::Exact(vec![type_id]);

        if register_fallback_first {
            registry
                .register_function(
                    "identity",
                    FunctionSignature::AnySingle,
                    Some(type_id),
                    execute_function,
                )
                .unwrap();
        }
        registry
            .register_function(
                "identity",
                exact_signature.clone(),
                Some(type_id),
                execute_function,
            )
            .unwrap();
        if !register_fallback_first {
            registry
                .register_function(
                    "identity",
                    FunctionSignature::AnySingle,
                    Some(type_id),
                    execute_function,
                )
                .unwrap();
        }

        let resolved = registry.resolve_function("identity", &[type_id]).unwrap();
        assert!(matches!(
            &registry.function(resolved).unwrap().signature,
            FunctionSignature::Exact(types) if types == &[type_id]
        ));

        assert!(matches!(
            registry.register_function(
                "identity",
                exact_signature,
                Some(type_id),
                execute_function,
            ),
            Err(CoreError::DuplicateFunctionSignature { name, signature })
                if name == "identity" && signature == "(int)"
        ));
        assert!(matches!(
            registry.register_function(
                "identity",
                FunctionSignature::AnySingle,
                Some(type_id),
                execute_function,
            ),
            Err(CoreError::DuplicateFunctionSignature { name, signature })
                if name == "identity" && signature == "(any)"
        ));
    }
}
