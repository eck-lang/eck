use super::*;

use language_core::Registry;

/// Verifies integer multiplication and checked overflow handling.
#[test]
fn multiplies_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer16::test_type_id(), 6_i16);
    let rhs = Value::new(crate::integer::integer16::test_type_id(), 7_i16);
    let maximum = Value::new(crate::integer::integer16::test_type_id(), i16::MAX);
    let two = Value::new(crate::integer::integer16::test_type_id(), 2_i16);

    let result = multiplication_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i16>().unwrap(), 42);
    assert!(matches!(
        multiplication_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed multiplication promotes both orders and checks `int16` overflow.
#[test]
fn multiplies_promoted_integer8_operands_as_integer16() {
    let wider_id = crate::integer::integer16::test_type_id();
    let narrower_id = crate::integer::integer8::test_type_id();
    let maximum = Value::new(wider_id, i16::MAX);
    let one = Value::new(narrower_id, 1_i8);
    let two = Value::new(narrower_id, 2_i8);

    for result in [
        multiplication_mixed_integer(&maximum, &one).unwrap(),
        multiplication_mixed_integer(&one, &maximum).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<i16>().unwrap(), i16::MAX);
    }
    assert!(matches!(
        multiplication_mixed_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
    let invalid = Value::new(narrower_id, false);
    assert!(matches!(
        multiplication_mixed_integer(&invalid, &maximum),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}

/// Verifies context-aware multiplication promotes `int16` overflow to `int32`.
#[test]
fn promotes_overflowed_context_multiplication_to_int32() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer16_id = registry.type_by_name("int16").unwrap();
    let int32_id = registry.type_by_name("int32").unwrap();
    let maximum = Value::new(integer16_id, i16::MAX);
    let two = Value::new(integer16_id, 2_i16);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Multiplication, integer16_id, integer16_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &maximum, &two).unwrap();

    assert_eq!(promoted.type_id(), int32_id);
    assert_eq!(*promoted.downcast_ref::<i32>().unwrap(), 65_534);
}
