use super::*;

use super::super::test_support::{execute_function, foreign_type_id, register_type};

/// Keeps undeclared native functions conservative and preserves explicit summaries by ID.
#[test]
fn function_effect_summaries_default_to_unknown_and_accept_declarations() {
    let mut registry = Registry::new();
    let type_id = register_type(&mut registry, "int");
    let unknown = registry
        .register_function(
            "unknown",
            FunctionSignature::Exact(vec![type_id]),
            Some(type_id),
            execute_function,
        )
        .unwrap();
    assert_eq!(
        registry.function(unknown).unwrap().effect_summary,
        FunctionEffectSummary::UNKNOWN
    );

    let pure = registry
        .register_function_with_effect_summary(
            "pure",
            FunctionSignature::Exact(vec![type_id]),
            Some(type_id),
            execute_function,
            FunctionEffectSummary::PURE,
        )
        .unwrap();
    assert_eq!(
        registry.function(pure).unwrap().effect_summary,
        FunctionEffectSummary::PURE
    );

    let impure_summary = FunctionEffectSummary {
        purity: crate::semantic::FunctionPurity::Impure,
        determinism: crate::semantic::FunctionDeterminism::Deterministic,
        may_fail: true,
        external_effect: crate::semantic::FunctionExternalEffect::WritesExternalState,
    };
    let impure = registry
        .register_function_with_effect_summary(
            "impure",
            FunctionSignature::Exact(vec![type_id]),
            None,
            execute_function,
            impure_summary,
        )
        .unwrap();
    assert_eq!(
        registry.function(impure).unwrap().effect_summary,
        impure_summary
    );
}

/// Audits the effect declarations on built-in native functions.
#[test]
fn builtin_function_effect_summaries_match_their_observable_behavior() {
    use crate::semantic::{
        FunctionDeterminism, FunctionExternalEffect, FunctionPurity, default_registry,
    };

    let registry = default_registry().unwrap();
    let string_type = registry.type_by_name("string").unwrap();
    let uppercase = registry
        .resolve_function("String.uppercase", &[string_type])
        .unwrap();
    assert_eq!(
        registry.function(uppercase).unwrap().effect_summary,
        FunctionEffectSummary {
            purity: FunctionPurity::Pure,
            determinism: FunctionDeterminism::Deterministic,
            may_fail: true,
            external_effect: FunctionExternalEffect::None,
        }
    );
    let repeat = registry
        .resolve_function(
            "String.repeat",
            &[string_type, registry.type_by_name("int").unwrap()],
        )
        .unwrap();
    assert_eq!(
        registry.function(repeat).unwrap().effect_summary,
        FunctionEffectSummary {
            purity: FunctionPurity::Pure,
            determinism: FunctionDeterminism::Deterministic,
            may_fail: true,
            external_effect: FunctionExternalEffect::None,
        }
    );

    let formatting = registry.resolve_any_single_function("string").unwrap();
    assert_eq!(
        registry.function(formatting).unwrap().effect_summary,
        FunctionEffectSummary::UNKNOWN
    );

    let print = registry.resolve_any_single_function("print").unwrap();
    assert_eq!(
        registry.function(print).unwrap().effect_summary.purity,
        FunctionPurity::Impure
    );
    assert_eq!(
        registry
            .function(print)
            .unwrap()
            .effect_summary
            .external_effect,
        FunctionExternalEffect::WritesExternalState
    );
    assert!(registry.function(print).unwrap().effect_summary.may_fail);

    let boolean_type = registry.type_by_name("bool").unwrap();
    let csv = registry
        .resolve_function(
            "CSV.read",
            &[
                string_type,
                string_type,
                boolean_type,
                string_type,
                string_type,
                boolean_type,
            ],
        )
        .unwrap();
    assert_eq!(
        registry.function(csv).unwrap().effect_summary.purity,
        FunctionPurity::Impure
    );
    assert_eq!(
        registry
            .function(csv)
            .unwrap()
            .effect_summary
            .external_effect,
        FunctionExternalEffect::ReadsExternalState
    );
}

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
