use super::*;

use language_core::Registry;

/// Verifies integer subtraction and checked overflow handling.
#[test]
fn subtracts_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer32::test_type_id(), 15_i32);
    let rhs = Value::new(crate::integer::integer32::test_type_id(), 27_i32);
    let minimum = Value::new(crate::integer::integer32::test_type_id(), i32::MIN);
    let one = Value::new(crate::integer::integer32::test_type_id(), 1_i32);

    let result = subtraction_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i32>().unwrap(), -12);
    assert!(matches!(
        subtraction_integer(&minimum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed subtraction preserves order and checked `int32` boundaries.
#[test]
fn subtracts_promoted_integer16_operands_as_integer32() {
    let wider_id = crate::integer::integer32::test_type_id();
    let narrower_id = crate::integer::integer16::test_type_id();
    let minimum = Value::new(wider_id, i32::MIN);
    let negative_one = Value::new(narrower_id, -1_i16);
    let one = Value::new(narrower_id, 1_i16);

    let wide_left = subtraction_mixed_integer(&minimum, &negative_one).unwrap();
    let narrow_left = subtraction_mixed_integer(&negative_one, &minimum).unwrap();
    assert_eq!(wide_left.type_id(), wider_id);
    assert_eq!(*wide_left.downcast_ref::<i32>().unwrap(), i32::MIN + 1);
    assert_eq!(narrow_left.type_id(), wider_id);
    assert_eq!(*narrow_left.downcast_ref::<i32>().unwrap(), i32::MAX);
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

/// Verifies context-aware subtraction promotes `int32` overflow to `int64`.
#[test]
fn promotes_overflowed_context_subtraction_to_int64() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer32_id = registry.type_by_name("int32").unwrap();
    let integer64_id = registry.type_by_name("int64").unwrap();
    let minimum = Value::new(integer32_id, i32::MIN);
    let one = Value::new(integer32_id, 1_i32);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Subtraction, integer32_id, integer32_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &minimum, &one).unwrap();

    assert_eq!(promoted.type_id(), integer64_id);
    assert_eq!(*promoted.downcast_ref::<i64>().unwrap(), -2_147_483_649);
}
