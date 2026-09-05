use super::*;

use language_core::Registry;

/// Verifies integer multiplication and checked overflow handling.
#[test]
fn multiplies_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer32::test_type_id(), 6_i32);
    let rhs = Value::new(crate::integer::integer32::test_type_id(), 7_i32);
    let maximum = Value::new(crate::integer::integer32::test_type_id(), i32::MAX);
    let two = Value::new(crate::integer::integer32::test_type_id(), 2_i32);

    let result = multiplication_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i32>().unwrap(), 42);
    assert!(matches!(
        multiplication_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed multiplication promotes both orders and checks `int32` overflow.
#[test]
fn multiplies_promoted_integer16_operands_as_integer32() {
    let wider_id = crate::integer::integer32::test_type_id();
    let narrower_id = crate::integer::integer16::test_type_id();
    let maximum = Value::new(wider_id, i32::MAX);
    let one = Value::new(narrower_id, 1_i16);
    let two = Value::new(narrower_id, 2_i16);

    for result in [
        multiplication_mixed_integer(&maximum, &one).unwrap(),
        multiplication_mixed_integer(&one, &maximum).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<i32>().unwrap(), i32::MAX);
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

/// Verifies context-aware multiplication promotes `int32` overflow to `int64`.
#[test]
fn promotes_overflowed_context_multiplication_to_int64() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer32_id = registry.type_by_name("int32").unwrap();
    let integer64_id = registry.type_by_name("int64").unwrap();
    let maximum = Value::new(integer32_id, i32::MAX);
    let two = Value::new(integer32_id, 2_i32);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Multiplication, integer32_id, integer32_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &maximum, &two).unwrap();

    assert_eq!(promoted.type_id(), integer64_id);
    assert_eq!(*promoted.downcast_ref::<i64>().unwrap(), 4_294_967_294);
}
