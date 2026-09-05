use super::*;

use language_core::Registry;

/// Verifies integer remainder, zero-divisor rejection, and checked overflow.
#[test]
fn calculates_integer_remainder_and_rejects_zero_and_overflow() {
    let lhs = Value::new(crate::integer::integer8::test_type_id(), 43_i8);
    let rhs = Value::new(crate::integer::integer8::test_type_id(), 5_i8);
    let zero = Value::new(crate::integer::integer8::test_type_id(), 0_i8);
    let minimum = Value::new(crate::integer::integer8::test_type_id(), i8::MIN);
    let negative_one = Value::new(crate::integer::integer8::test_type_id(), -1_i8);

    let result = remainder_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i8>().unwrap(), 3);
    assert!(matches!(
        remainder_integer(&lhs, &zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        remainder_integer(&minimum, &negative_one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies context-aware remainder promotes the `MIN % -1` overflow to `int16`.
#[test]
fn promotes_overflowed_context_remainder_to_int16() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer8_id = registry.type_by_name("int8").unwrap();
    let int16_id = registry.type_by_name("int16").unwrap();
    let minimum = Value::new(integer8_id, i8::MIN);
    let negative_one = Value::new(integer8_id, -1_i8);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Remainder, integer8_id, integer8_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &minimum, &negative_one).unwrap();

    assert_eq!(promoted.type_id(), int16_id);
    assert_eq!(*promoted.downcast_ref::<i16>().unwrap(), 0);
}
