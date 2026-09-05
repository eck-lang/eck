use super::*;

use language_core::Registry;

/// Builds a fully registered registry with its default configuration.
///
/// Callers build their own execution context borrowing both values, so they
/// stay alive for the duration of the test. Full registration matters
/// because overflow promotion resolves `int32` by name.
fn registered_registry_with_configuration() -> (Registry, language_core::RuntimeConfiguration) {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    (registry, configuration)
}

/// Verifies integer addition and checked overflow handling.
#[test]
fn adds_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer16::test_type_id(), 15_i16);
    let rhs = Value::new(crate::integer::integer16::test_type_id(), 27_i16);
    let maximum = Value::new(crate::integer::integer16::test_type_id(), i16::MAX);
    let one = Value::new(crate::integer::integer16::test_type_id(), 1_i16);

    let result = addition_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i16>().unwrap(), 42);
    assert!(matches!(
        addition_integer(&maximum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed addition promotes both orders and keeps `int16` boundary errors.
#[test]
fn adds_promoted_integer8_operands_as_integer16() {
    let wider_id = crate::integer::integer16::test_type_id();
    let narrower_id = crate::integer::integer8::test_type_id();
    let maximum = Value::new(wider_id, i16::MAX);
    let negative_one = Value::new(narrower_id, -1_i8);
    let one = Value::new(narrower_id, 1_i8);

    for result in [
        addition_mixed_integer(&maximum, &negative_one).unwrap(),
        addition_mixed_integer(&negative_one, &maximum).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<i16>().unwrap(), i16::MAX - 1);
    }
    assert!(matches!(
        addition_mixed_integer(&maximum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
    let invalid = Value::new(narrower_id, false);
    assert!(matches!(
        addition_mixed_integer(&invalid, &maximum),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}

/// Verifies context-aware addition keeps in-range results as `int16`.
#[test]
fn keeps_in_range_context_addition_as_integer16() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer16_id = registry.type_by_name("int16").unwrap();
    let lhs = Value::new(integer16_id, 15_i16);
    let rhs = Value::new(integer16_id, 27_i16);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Addition, integer16_id, integer16_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let result = descriptor.context_execute.unwrap()(&context, &lhs, &rhs).unwrap();

    assert_eq!(result.type_id(), integer16_id);
    assert_eq!(*result.downcast_ref::<i16>().unwrap(), 42);
}

/// Verifies context-aware addition promotes `int16` overflow to `int32`.
#[test]
fn promotes_overflowed_context_addition_to_int32() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer16_id = registry.type_by_name("int16").unwrap();
    let int32_id = registry.type_by_name("int32").unwrap();
    let maximum = Value::new(integer16_id, i16::MAX);
    let one = Value::new(integer16_id, 1_i16);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Addition, integer16_id, integer16_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &maximum, &one).unwrap();

    assert_eq!(promoted.type_id(), int32_id);
    assert_eq!(*promoted.downcast_ref::<i32>().unwrap(), 32_768);
}

/// Verifies context-aware mixed addition promotes `int16` overflow to `int32`.
#[test]
fn promotes_overflowed_mixed_context_addition_to_int32() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer16_id = registry.type_by_name("int16").unwrap();
    let integer8_id = registry.type_by_name("int8").unwrap();
    let int32_id = registry.type_by_name("int32").unwrap();
    let maximum = Value::new(integer16_id, i16::MAX);
    let one = Value::new(integer8_id, 1_i8);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Addition, integer16_id, integer8_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &maximum, &one).unwrap();

    assert_eq!(promoted.type_id(), int32_id);
    assert_eq!(*promoted.downcast_ref::<i32>().unwrap(), 32_768);
}
