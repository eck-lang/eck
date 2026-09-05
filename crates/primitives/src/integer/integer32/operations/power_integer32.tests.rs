use super::*;

use language_core::Registry;

/// Verifies integer power, exponent validation, and checked overflow handling.
#[test]
fn raises_integers_to_non_negative_powers_and_rejects_invalid_exponents() {
    let base = Value::new(crate::integer::integer32::test_type_id(), 2_i32);
    let exponent = Value::new(crate::integer::integer32::test_type_id(), 6_i32);
    let negative = Value::new(crate::integer::integer32::test_type_id(), -1_i32);
    let maximum = Value::new(crate::integer::integer32::test_type_id(), i32::MAX);
    let two = Value::new(crate::integer::integer32::test_type_id(), 2_i32);

    let result = power_integer(&base, &exponent).unwrap();

    assert_eq!(*result.downcast_ref::<i32>().unwrap(), 64);
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
fn powers_promoted_integer16_operands_as_integer32() {
    let wider_id = crate::integer::integer32::test_type_id();
    let narrower_id = crate::integer::integer16::test_type_id();
    let wide_two = Value::new(wider_id, 2_i32);
    let narrow_two = Value::new(narrower_id, 2_i16);
    let wide_three = Value::new(wider_id, 3_i32);
    let narrow_three = Value::new(narrower_id, 3_i16);

    for result in [
        power_mixed_integer(&wide_two, &narrow_three).unwrap(),
        power_mixed_integer(&narrow_two, &wide_three).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<i32>().unwrap(), 8);
    }
    let maximum = Value::new(wider_id, i32::MAX);
    assert!(
        matches!(power_mixed_integer(&maximum, &narrow_two), Err(CoreError::Runtime(message)) if message.contains("overflow"))
    );
    let negative = Value::new(narrower_id, -1_i16);
    assert!(
        matches!(power_mixed_integer(&wide_two, &negative), Err(CoreError::Runtime(message)) if message.contains("non-negative"))
    );
    let invalid = Value::new(narrower_id, false);
    assert!(matches!(
        power_mixed_integer(&invalid, &wide_two),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}

/// Verifies context-aware power promotes `int32` overflow to `int64`.
#[test]
fn promotes_overflowed_context_power_to_int64() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer32_id = registry.type_by_name("int32").unwrap();
    let integer64_id = registry.type_by_name("int64").unwrap();
    let base = Value::new(integer32_id, 2_i32);
    let exponent = Value::new(integer32_id, 31_i32);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Power, integer32_id, integer32_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &base, &exponent).unwrap();

    assert_eq!(promoted.type_id(), integer64_id);
    assert_eq!(*promoted.downcast_ref::<i64>().unwrap(), 2_147_483_648);
}

/// Verifies context-aware power still reports overflow beyond `int64`.
#[test]
fn keeps_context_power_overflow_beyond_int64_as_error() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer32_id = registry.type_by_name("int32").unwrap();
    let base = Value::new(integer32_id, 100_i32);
    let exponent = Value::new(integer32_id, 10_i32);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Power, integer32_id, integer32_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    assert!(matches!(
        descriptor.context_execute.unwrap()(&context, &base, &exponent),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
