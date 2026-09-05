use super::*;

use language_core::Registry;

/// Builds a fully registered registry with its default configuration.
///
/// Callers build their own execution context borrowing both values, so they
/// stay alive for the duration of the test. Full registration matters
/// because overflow promotion resolves `int16` by name.
fn registered_registry_with_configuration() -> (Registry, language_core::RuntimeConfiguration) {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    (registry, configuration)
}

/// Verifies integer addition and checked overflow handling.
#[test]
fn adds_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer8::test_type_id(), 15_i8);
    let rhs = Value::new(crate::integer::integer8::test_type_id(), 27_i8);
    let maximum = Value::new(crate::integer::integer8::test_type_id(), i8::MAX);
    let one = Value::new(crate::integer::integer8::test_type_id(), 1_i8);

    let result = addition_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i8>().unwrap(), 42);
    assert!(matches!(
        addition_integer(&maximum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies context-aware addition keeps in-range results as `int8`.
#[test]
fn keeps_in_range_context_addition_as_integer8() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer8_id = registry.type_by_name("int8").unwrap();
    let lhs = Value::new(integer8_id, 15_i8);
    let rhs = Value::new(integer8_id, 27_i8);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Addition, integer8_id, integer8_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let result = descriptor.context_execute.unwrap()(&context, &lhs, &rhs).unwrap();

    assert_eq!(result.type_id(), integer8_id);
    assert_eq!(*result.downcast_ref::<i8>().unwrap(), 42);
}

/// Verifies context-aware addition promotes `int8` overflow to `int16`.
#[test]
fn promotes_overflowed_context_addition_to_int16() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer8_id = registry.type_by_name("int8").unwrap();
    let int16_id = registry.type_by_name("int16").unwrap();
    let maximum = Value::new(integer8_id, i8::MAX);
    let one = Value::new(integer8_id, 1_i8);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Addition, integer8_id, integer8_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &maximum, &one).unwrap();

    assert_eq!(promoted.type_id(), int16_id);
    assert_eq!(*promoted.downcast_ref::<i16>().unwrap(), 128);
}
