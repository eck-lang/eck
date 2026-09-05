use super::*;

use language_core::Registry;
use num_bigint::BigInt;

/// Verifies integer addition and checked overflow handling.
#[test]
fn adds_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer128::test_type_id(), 15_i128);
    let rhs = Value::new(crate::integer::integer128::test_type_id(), 27_i128);
    let maximum = Value::new(crate::integer::integer128::test_type_id(), i128::MAX);
    let one = Value::new(crate::integer::integer128::test_type_id(), 1_i128);

    let result = addition_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i128>().unwrap(), 42);
    assert!(matches!(
        addition_integer(&maximum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed addition promotes both orders and keeps `int128` boundary errors.
#[test]
fn adds_promoted_integer64_operands_as_integer128() {
    let wider_id = crate::integer::integer128::test_type_id();
    let narrower_id = crate::integer::integer64::test_type_id();
    let maximum = Value::new(wider_id, i128::MAX);
    let negative_one = Value::new(narrower_id, -1_i64);
    let one = Value::new(narrower_id, 1_i64);

    for result in [
        addition_mixed_integer(&maximum, &negative_one).unwrap(),
        addition_mixed_integer(&negative_one, &maximum).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<i128>().unwrap(), i128::MAX - 1);
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

/// Builds a fully registered registry with its default configuration.
///
/// Callers build their own execution context borrowing both values, so they
/// stay alive for the duration of the test. Full registration matters
/// because overflow promotion resolves `bigint` by name.
fn registered_registry_with_configuration() -> (Registry, language_core::RuntimeConfiguration) {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    (registry, configuration)
}

/// Verifies context-aware addition keeps in-range results as `int128`.
#[test]
fn keeps_in_range_context_addition_as_integer128() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer128_id = registry.type_by_name("int128").unwrap();
    let lhs = Value::new(integer128_id, 15_i128);
    let rhs = Value::new(integer128_id, 27_i128);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Addition, integer128_id, integer128_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let result = descriptor.context_execute.unwrap()(&context, &lhs, &rhs).unwrap();

    assert_eq!(result.type_id(), integer128_id);
    assert_eq!(*result.downcast_ref::<i128>().unwrap(), 42);
}

/// Verifies context-aware addition promotes `int128` overflow to `bigint`.
#[test]
fn promotes_overflowed_context_addition_to_bigint() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer128_id = registry.type_by_name("int128").unwrap();
    let bigint_id = registry.type_by_name("bigint").unwrap();
    let maximum = Value::new(integer128_id, i128::MAX);
    let one = Value::new(integer128_id, 1_i128);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Addition, integer128_id, integer128_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &maximum, &one).unwrap();

    assert_eq!(promoted.type_id(), bigint_id);
    assert_eq!(
        promoted.downcast_ref::<BigInt>().unwrap(),
        &(BigInt::from(i128::MAX) + 1)
    );
}

/// Verifies context-aware mixed addition promotes `int128` overflow to `bigint`.
#[test]
fn promotes_overflowed_mixed_context_addition_to_bigint() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer128_id = registry.type_by_name("int128").unwrap();
    let integer64_id = registry.type_by_name("int64").unwrap();
    let bigint_id = registry.type_by_name("bigint").unwrap();
    let maximum = Value::new(integer128_id, i128::MAX);
    let one = Value::new(integer64_id, 1_i64);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Addition, integer128_id, integer64_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &maximum, &one).unwrap();

    assert_eq!(promoted.type_id(), bigint_id);
    assert_eq!(
        promoted.downcast_ref::<BigInt>().unwrap(),
        &(BigInt::from(i128::MAX) + 1)
    );
}
