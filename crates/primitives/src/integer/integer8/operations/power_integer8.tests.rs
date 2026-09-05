use super::*;

use language_core::Registry;

/// Verifies integer power, exponent validation, and checked overflow handling.
#[test]
fn raises_integers_to_non_negative_powers_and_rejects_invalid_exponents() {
    let base = Value::new(crate::integer::integer8::test_type_id(), 2_i8);
    let exponent = Value::new(crate::integer::integer8::test_type_id(), 6_i8);
    let negative = Value::new(crate::integer::integer8::test_type_id(), -1_i8);
    let maximum = Value::new(crate::integer::integer8::test_type_id(), i8::MAX);
    let two = Value::new(crate::integer::integer8::test_type_id(), 2_i8);

    let result = power_integer(&base, &exponent).unwrap();

    assert_eq!(*result.downcast_ref::<i8>().unwrap(), 64);
    assert!(matches!(
        power_integer(&base, &negative),
        Err(CoreError::Runtime(message)) if message.contains("non-negative")
    ));
    assert!(matches!(
        power_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies context-aware power promotes `int8` overflow to `int16`.
#[test]
fn promotes_overflowed_context_power_to_int16() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer8_id = registry.type_by_name("int8").unwrap();
    let int16_id = registry.type_by_name("int16").unwrap();
    let base = Value::new(integer8_id, 2_i8);
    let exponent = Value::new(integer8_id, 7_i8);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Power, integer8_id, integer8_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &base, &exponent).unwrap();

    assert_eq!(promoted.type_id(), int16_id);
    assert_eq!(*promoted.downcast_ref::<i16>().unwrap(), 128);
}

/// Verifies context-aware power still reports overflow beyond `int16`.
#[test]
fn keeps_context_power_overflow_beyond_int16_as_error() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer8_id = registry.type_by_name("int8").unwrap();
    let base = Value::new(integer8_id, 10_i8);
    let exponent = Value::new(integer8_id, 10_i8);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Power, integer8_id, integer8_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    assert!(matches!(
        descriptor.context_execute.unwrap()(&context, &base, &exponent),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}
