use super::*;

use language_core::Registry;

/// Builds a fully registered registry for context-aware promotion tests.
fn registered_registry_with_configuration() -> (Registry, language_core::RuntimeConfiguration) {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    (registry, configuration)
}

/// Verifies integer subtraction and checked overflow handling.
#[test]
fn subtracts_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer64::test_type_id(), 15_i64);
    let rhs = Value::new(crate::integer::integer64::test_type_id(), 27_i64);
    let minimum = Value::new(crate::integer::integer64::test_type_id(), i64::MIN);
    let one = Value::new(crate::integer::integer64::test_type_id(), 1_i64);

    let result = subtraction_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i64>().unwrap(), -12);
    assert!(matches!(
        subtraction_integer(&minimum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed subtraction preserves order and checked `int64` boundaries.
#[test]
fn subtracts_promoted_integer32_operands_as_integer64() {
    let wider_id = crate::integer::integer64::test_type_id();
    let narrower_id = crate::integer::integer32::test_type_id();
    let minimum = Value::new(wider_id, i64::MIN);
    let negative_one = Value::new(narrower_id, -1_i32);
    let one = Value::new(narrower_id, 1_i32);

    let wide_left = subtraction_mixed_integer(&minimum, &negative_one).unwrap();
    let narrow_left = subtraction_mixed_integer(&negative_one, &minimum).unwrap();
    assert_eq!(wide_left.type_id(), wider_id);
    assert_eq!(*wide_left.downcast_ref::<i64>().unwrap(), i64::MIN + 1);
    assert_eq!(narrow_left.type_id(), wider_id);
    assert_eq!(*narrow_left.downcast_ref::<i64>().unwrap(), i64::MAX);
    assert!(matches!(
        subtraction_mixed_integer(&minimum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
    let invalid = Value::new(narrower_id, false);
    assert!(matches!(
        subtraction_mixed_integer(&invalid, &minimum),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}

/// Verifies context-aware subtraction promotes `int64` overflow to `int128`.
#[test]
fn promotes_overflowed_context_subtraction_to_int128() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer64_id = registry.type_by_name("int64").unwrap();
    let integer128_id = registry.type_by_name("int128").unwrap();
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Subtraction, integer64_id, integer64_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();
    let execute = descriptor.context_execute.unwrap();

    let in_range = execute(
        &context,
        &Value::new(integer64_id, 27_i64),
        &Value::new(integer64_id, 15_i64),
    )
    .unwrap();
    assert_eq!(in_range.type_id(), integer64_id);
    assert_eq!(*in_range.downcast_ref::<i64>().unwrap(), 12);

    let promoted = execute(
        &context,
        &Value::new(integer64_id, i64::MIN),
        &Value::new(integer64_id, 1_i64),
    )
    .unwrap();
    assert_eq!(promoted.type_id(), integer128_id);
    assert_eq!(
        *promoted.downcast_ref::<i128>().unwrap(),
        -9_223_372_036_854_775_809_i128
    );
}

/// Verifies context-aware mixed subtraction promotes `int64` overflow to `int128`.
#[test]
fn promotes_overflowed_mixed_context_subtraction_to_int128() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer64_id = registry.type_by_name("int64").unwrap();
    let integer32_id = registry.type_by_name("int32").unwrap();
    let integer128_id = registry.type_by_name("int128").unwrap();
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Subtraction, integer64_id, integer32_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(
        &context,
        &Value::new(integer64_id, i64::MIN),
        &Value::new(integer32_id, 1_i32),
    )
    .unwrap();
    assert_eq!(promoted.type_id(), integer128_id);
    assert_eq!(
        *promoted.downcast_ref::<i128>().unwrap(),
        -9_223_372_036_854_775_809_i128
    );
}
