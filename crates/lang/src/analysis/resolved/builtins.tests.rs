use super::*;
use crate::semantic::{
    BinaryOperator, CoreError, RuntimeConfiguration, TypeConfigurationDescriptor, TypeDescriptor,
};

/// The canonical builtin registry verifies actual arithmetic and index callbacks.
#[test]
fn builtin_callbacks_are_verified() {
    let registry = crate::semantic::default_registry().unwrap();
    let builtins = Builtins::new(&registry);
    let integer = registry.default_integer().unwrap();
    let addition = registry
        .resolve_binary_operator(BinaryOperator::Addition, integer, integer)
        .unwrap();
    assert!(builtins.operator(addition));
    assert!(builtins.callbacks_are_trusted());
    assert!(builtins.index_extractor(integer, registry.index_extractor(integer).unwrap()));
}

/// A counterfeit builtin name and even a same-layout payload are insufficient evidence.
#[test]
fn counterfeit_integer_parser_is_untrusted() {
    let canonical = crate::semantic::default_registry().unwrap();
    let mut registry = Registry::new();
    let mut descriptor = canonical
        .type_descriptor(canonical.default_integer().unwrap())
        .unwrap()
        .clone();
    descriptor.id = registry.allocate_type_id();
    descriptor.parse_numeric_literal = Some(counterfeit_integer);
    let integer = descriptor.id;
    registry.register_type(descriptor).unwrap();
    registry.set_default_integer(integer).unwrap();
    let operator = registry
        .register_binary_operator(
            BinaryOperator::Addition,
            integer,
            integer,
            integer,
            counterfeit_addition,
        )
        .unwrap();
    let builtins = Builtins::new(&registry);
    assert!(builtins.canonical_type(integer).is_none());
    assert!(!builtins.operator(operator));
}

/// Matching builtin types cannot legitimize a replacement arithmetic executor.
#[test]
fn counterfeit_operator_on_real_integer_descriptor_is_untrusted() {
    let canonical = crate::semantic::default_registry().unwrap();
    let mut registry = Registry::new();
    let mut descriptor = canonical
        .type_descriptor(canonical.default_integer().unwrap())
        .unwrap()
        .clone();
    descriptor.id = registry.allocate_type_id();
    let integer = descriptor.id;
    registry.register_type(descriptor).unwrap();
    registry.set_default_integer(integer).unwrap();
    let operator = registry
        .register_binary_operator(
            BinaryOperator::Addition,
            integer,
            integer,
            integer,
            counterfeit_addition,
        )
        .unwrap();
    let builtins = Builtins::new(&registry);
    assert!(builtins.canonical_type(integer).is_some());
    assert!(!builtins.operator_identity(operator));
}

/// Appending an effectful transformer to a builtin type invalidates its proof.
#[test]
fn appended_type_configuration_is_untrusted() {
    let mut registry = crate::semantic::default_registry().unwrap();
    let integer = registry.default_integer().unwrap();
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Addition, integer, integer)
        .unwrap();
    registry
        .register_type_configuration(
            integer,
            TypeConfigurationDescriptor {
                transform_result: Some(counterfeit_transform),
                transform_owned_result: None,
                initial_result_transform_is_identity: true,
                format: None,
            },
        )
        .unwrap();
    assert!(!Builtins::new(&registry).operator(operator));
}

/// Boolean evaluator replacement is rejected independently of the builtin bool name.
#[test]
fn replaced_boolean_evaluator_is_untrusted() {
    let mut registry = crate::semantic::default_registry().unwrap();
    registry
        .set_default_boolean(registry.default_boolean().unwrap(), counterfeit_boolean)
        .unwrap();
    assert!(!Builtins::new(&registry).callbacks_are_trusted());
}

/// Embedded extractors are compared by identity rather than integral capability.
#[test]
fn replaced_index_extractor_is_untrusted() {
    let registry = crate::semantic::default_registry().unwrap();
    let builtins = Builtins::new(&registry);
    assert!(!builtins.index_extractor(registry.default_integer().unwrap(), counterfeit_index));
}

/// Default fractional and string hooks cannot smuggle effects into builtin callbacks.
#[test]
fn changed_default_parsers_are_untrusted() {
    let mut registry = crate::semantic::default_registry().unwrap();
    let integer = registry.default_integer().unwrap();
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Addition, integer, integer)
        .unwrap();
    let custom = registry.allocate_type_id();
    registry
        .register_type(TypeDescriptor {
            id: custom,
            name: "external",
            is_integer: false,
            parse_numeric_literal: Some(counterfeit_integer),
            parse_string_literal: Some(counterfeit_string),
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: counterfeit_format,
        })
        .unwrap();
    registry.set_default_fractional(custom).unwrap();
    assert!(!Builtins::new(&registry).operator(operator));
    registry.set_default_string(custom).unwrap();
    let string = registry.type_by_name("string").unwrap();
    let function = registry
        .resolve_function("String.uppercase", &[string])
        .unwrap();
    assert!(!Builtins::new(&registry).function_callbacks(function));
}

/// Creates the same payload shape as int64 without being its trusted parser.
fn counterfeit_integer(text: &str, type_id: TypeId) -> Result<Value, CoreError> {
    Ok(Value::new(type_id, text.parse::<i64>().unwrap()))
}

/// Creates a string payload under an extension-controlled default identity.
fn counterfeit_string(text: &str, type_id: TypeId) -> Result<Value, CoreError> {
    Ok(Value::new(type_id, text.to_string()))
}

/// Represents an untrusted extension arithmetic callback.
fn counterfeit_addition(left: &Value, _: &Value) -> Result<Value, CoreError> {
    Ok(left.clone())
}

/// Represents an untrusted extension configuration hook.
fn counterfeit_transform(value: &Value, _: &RuntimeConfiguration) -> Result<Value, CoreError> {
    Ok(value.clone())
}

/// Represents an untrusted extension boolean evaluator.
fn counterfeit_boolean(_: &Value) -> Result<bool, CoreError> {
    Ok(true)
}

/// Represents an untrusted extension index extractor.
fn counterfeit_index(_: &Value) -> Result<usize, CoreError> {
    Ok(0)
}

/// Represents an untrusted extension formatting hook.
fn counterfeit_format(_: &Value) -> Result<String, CoreError> {
    Ok("0".into())
}

/// A fresh default registry proves independent writes without constructing a second inventory.
#[test]
fn certifies_only_fresh_default_registries() {
    let registry = crate::semantic::default_registry().unwrap();
    assert!(std::ptr::eq(Builtins::new(&registry).canonical, &registry));
    let program = crate::compile(
        &crate::parse("let output: int[] = [0,0,0]\nfor (i in 0..3) { output[i] = i }").unwrap(),
        &registry,
    )
    .unwrap();
    assert!(matches!(
        program
            .execution_analysis()
            .loops
            .values()
            .next()
            .unwrap()
            .parallelism,
        crate::analysis::Parallelism::IndependentIterations { .. }
    ));

    let mut custom = Registry::new();
    crate::semantic::register_all(&mut custom).unwrap();
    assert!(!custom.builtin_inventory_is_certified());
    assert!(!std::ptr::eq(Builtins::new(&custom).canonical, &custom));
}

/// Recompiling after a colliding index callback cannot reuse builtin certification.
#[test]
fn changed_index_extractor_rejects_independent_writes() {
    let mut registry = crate::semantic::default_registry().unwrap();
    let integer = registry.default_integer().unwrap();
    registry
        .register_index_extractor(integer, counterfeit_index)
        .unwrap();
    assert!(!registry.builtin_inventory_is_certified());
    let program = crate::compile(
        &crate::parse("let output: int[] = [0,0,0]\nfor (i in 0..3) { output[i] = i }").unwrap(),
        &registry,
    )
    .unwrap();
    assert!(matches!(
        program
            .execution_analysis()
            .loops
            .values()
            .next()
            .unwrap()
            .parallelism,
        crate::analysis::Parallelism::Sequential { .. }
    ));
}

/// An appended result hook must be verified against independent builtin callbacks.
#[test]
fn changed_configuration_hook_rejects_builtin_arithmetic() {
    let mut registry = crate::semantic::default_registry().unwrap();
    let integer = registry.default_integer().unwrap();
    registry
        .register_type_configuration(
            integer,
            TypeConfigurationDescriptor {
                transform_result: Some(counterfeit_transform),
                transform_owned_result: None,
                initial_result_transform_is_identity: true,
                format: None,
            },
        )
        .unwrap();
    assert!(!registry.builtin_inventory_is_certified());
    let program = crate::compile(
        &crate::parse("for (i in 0..3) { let value = i * 2 }").unwrap(),
        &registry,
    )
    .unwrap();
    assert!(matches!(
        program
            .execution_analysis()
            .loops
            .values()
            .next()
            .unwrap()
            .parallelism,
        crate::analysis::Parallelism::Sequential { .. }
    ));
}
