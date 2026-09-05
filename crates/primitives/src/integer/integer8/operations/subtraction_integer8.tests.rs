use super::*;

use language_core::Registry;

/// Verifies integer subtraction and checked overflow handling.
#[test]
fn subtracts_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer8::test_type_id(), 15_i8);
    let rhs = Value::new(crate::integer::integer8::test_type_id(), 27_i8);
    let minimum = Value::new(crate::integer::integer8::test_type_id(), i8::MIN);
    let one = Value::new(crate::integer::integer8::test_type_id(), 1_i8);

    let result = subtraction_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i8>().unwrap(), -12);
    assert!(matches!(
        subtraction_integer(&minimum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies context-aware subtraction promotes `int8` overflow to `int16`.
#[test]
fn promotes_overflowed_context_subtraction_to_int16() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer8_id = registry.type_by_name("int8").unwrap();
    let int16_id = registry.type_by_name("int16").unwrap();
    let minimum = Value::new(integer8_id, i8::MIN);
    let one = Value::new(integer8_id, 1_i8);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Subtraction, integer8_id, integer8_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &minimum, &one).unwrap();

    assert_eq!(promoted.type_id(), int16_id);
    assert_eq!(*promoted.downcast_ref::<i16>().unwrap(), -129);
}
