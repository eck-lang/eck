use super::*;

use language_core::Registry;

/// Verifies integer multiplication and checked overflow handling.
#[test]
fn multiplies_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer8::test_type_id(), 6_i8);
    let rhs = Value::new(crate::integer::integer8::test_type_id(), 7_i8);
    let maximum = Value::new(crate::integer::integer8::test_type_id(), i8::MAX);
    let two = Value::new(crate::integer::integer8::test_type_id(), 2_i8);

    let result = multiplication_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i8>().unwrap(), 42);
    assert!(matches!(
        multiplication_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies context-aware multiplication promotes `int8` overflow to `int16`.
#[test]
fn promotes_overflowed_context_multiplication_to_int16() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer8_id = registry.type_by_name("int8").unwrap();
    let int16_id = registry.type_by_name("int16").unwrap();
    let maximum = Value::new(integer8_id, i8::MAX);
    let two = Value::new(integer8_id, 2_i8);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Multiplication, integer8_id, integer8_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &maximum, &two).unwrap();

    assert_eq!(promoted.type_id(), int16_id);
    assert_eq!(*promoted.downcast_ref::<i16>().unwrap(), 254);
}
