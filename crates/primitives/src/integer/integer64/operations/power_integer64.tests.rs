use super::*;

use language_core::Registry;

/// Builds a fully registered registry for context-aware promotion tests.
fn registered_registry_with_configuration() -> (Registry, language_core::RuntimeConfiguration) {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    (registry, configuration)
}

/// Verifies integer power, exponent validation, and checked overflow handling.
#[test]
fn raises_integers_to_non_negative_powers_and_rejects_invalid_exponents() {
    let base = Value::new(crate::integer::integer64::test_type_id(), 2_i64);
    let exponent = Value::new(crate::integer::integer64::test_type_id(), 10_i64);
    let negative = Value::new(crate::integer::integer64::test_type_id(), -1_i64);
    let maximum = Value::new(crate::integer::integer64::test_type_id(), i64::MAX);
    let two = Value::new(crate::integer::integer64::test_type_id(), 2_i64);

    let result = power_integer(&base, &exponent).unwrap();

    assert_eq!(*result.downcast_ref::<i64>().unwrap(), 1024);
    assert!(matches!(
        power_integer(&base, &negative),
        Err(CoreError::Runtime(message)) if message.contains("non-negative")
    ));
    assert!(matches!(
        power_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed power promotes both orders and preserves exponent and overflow errors.
#[test]
fn powers_promoted_integer32_operands_as_integer64() {
    let wider_id = crate::integer::integer64::test_type_id();
    let narrower_id = crate::integer::integer32::test_type_id();
    let wide_two = Value::new(wider_id, 2_i64);
    let narrow_two = Value::new(narrower_id, 2_i32);
    let wide_three = Value::new(wider_id, 3_i64);
    let narrow_three = Value::new(narrower_id, 3_i32);

    for result in [
        power_mixed_integer(&wide_two, &narrow_three).unwrap(),
        power_mixed_integer(&narrow_two, &wide_three).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<i64>().unwrap(), 8);
    }
    let maximum = Value::new(wider_id, i64::MAX);
    assert!(
        matches!(power_mixed_integer(&maximum, &narrow_two), Err(CoreError::Runtime(message)) if message.contains("overflow"))
    );
    let negative = Value::new(narrower_id, -1_i32);
    assert!(
        matches!(power_mixed_integer(&wide_two, &negative), Err(CoreError::Runtime(message)) if message.contains("non-negative"))
    );
    let invalid = Value::new(narrower_id, false);
    assert!(matches!(
        power_mixed_integer(&invalid, &wide_two),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}

/// Verifies context-aware power promotes `int64` overflow to `int128`.
#[test]
fn promotes_overflowed_context_power_to_int128() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer64_id = registry.type_by_name("int64").unwrap();
    let integer128_id = registry.type_by_name("int128").unwrap();
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Power, integer64_id, integer64_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();
    let execute = descriptor.context_execute.unwrap();

    let in_range = execute(
        &context,
        &Value::new(integer64_id, 2_i64),
        &Value::new(integer64_id, 5_i64),
    )
    .unwrap();
    assert_eq!(in_range.type_id(), integer64_id);
    assert_eq!(*in_range.downcast_ref::<i64>().unwrap(), 32);

    let promoted = execute(
        &context,
        &Value::new(integer64_id, 2_i64),
        &Value::new(integer64_id, 63_i64),
    )
    .unwrap();
    assert_eq!(promoted.type_id(), integer128_id);
    assert_eq!(
        *promoted.downcast_ref::<i128>().unwrap(),
        9_223_372_036_854_775_808_i128
    );

    assert!(matches!(
        execute(
            &context,
            &Value::new(integer64_id, 2_i64),
            &Value::new(integer64_id, -1_i64),
        ),
        Err(CoreError::Runtime(message)) if message.contains("non-negative")
    ));
}

/// Verifies context-aware mixed power promotes `int64` overflow to `int128`.
#[test]
fn promotes_overflowed_mixed_context_power_to_int128() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer64_id = registry.type_by_name("int64").unwrap();
    let integer32_id = registry.type_by_name("int32").unwrap();
    let integer128_id = registry.type_by_name("int128").unwrap();
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Power, integer64_id, integer32_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(
        &context,
        &Value::new(integer64_id, 2_i64),
        &Value::new(integer32_id, 63_i32),
    )
    .unwrap();
    assert_eq!(promoted.type_id(), integer128_id);
    assert_eq!(
        *promoted.downcast_ref::<i128>().unwrap(),
        9_223_372_036_854_775_808_i128
    );
}

/// Verifies context-aware power still reports overflow beyond `int128`.
#[test]
fn keeps_context_power_overflow_beyond_int128_as_error() {
    let (registry, configuration) = registered_registry_with_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer64_id = registry.type_by_name("int64").unwrap();
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Power, integer64_id, integer64_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    assert!(matches!(
        descriptor.context_execute.unwrap()(
            &context,
            &Value::new(integer64_id, 2_i64),
            &Value::new(integer64_id, 127_i64),
        ),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
