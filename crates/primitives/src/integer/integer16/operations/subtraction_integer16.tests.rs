use super::*;

use language_core::Registry;

/// Verifies integer subtraction and checked overflow handling.
#[test]
fn subtracts_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer16::test_type_id(), 15_i16);
    let rhs = Value::new(crate::integer::integer16::test_type_id(), 27_i16);
    let minimum = Value::new(crate::integer::integer16::test_type_id(), i16::MIN);
    let one = Value::new(crate::integer::integer16::test_type_id(), 1_i16);

    let result = subtraction_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i16>().unwrap(), -12);
    assert!(matches!(
        subtraction_integer(&minimum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed subtraction preserves order and checked `int16` boundaries.
#[test]
fn subtracts_promoted_integer8_operands_as_integer16() {
    let wider_id = crate::integer::integer16::test_type_id();
    let narrower_id = crate::integer::integer8::test_type_id();
    let minimum = Value::new(wider_id, i16::MIN);
    let negative_one = Value::new(narrower_id, -1_i8);
    let one = Value::new(narrower_id, 1_i8);

    let wide_left = subtraction_mixed_integer(&minimum, &negative_one).unwrap();
    let narrow_left = subtraction_mixed_integer(&negative_one, &minimum).unwrap();
    assert_eq!(wide_left.type_id(), wider_id);
    assert_eq!(*wide_left.downcast_ref::<i16>().unwrap(), i16::MIN + 1);
    assert_eq!(narrow_left.type_id(), wider_id);
    assert_eq!(*narrow_left.downcast_ref::<i16>().unwrap(), i16::MAX);
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

/// Verifies context-aware subtraction promotes `int16` overflow to `int32`.
#[test]
fn promotes_overflowed_context_subtraction_to_int32() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer16_id = registry.type_by_name("int16").unwrap();
    let int32_id = registry.type_by_name("int32").unwrap();
    let minimum = Value::new(integer16_id, i16::MIN);
    let one = Value::new(integer16_id, 1_i16);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Subtraction, integer16_id, integer16_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &minimum, &one).unwrap();

    assert_eq!(promoted.type_id(), int32_id);
    assert_eq!(*promoted.downcast_ref::<i32>().unwrap(), -32_769);
}
