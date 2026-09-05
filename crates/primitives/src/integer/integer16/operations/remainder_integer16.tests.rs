use super::*;

use language_core::Registry;

/// Verifies integer remainder, zero-divisor rejection, and checked overflow.
#[test]
fn calculates_integer_remainder_and_rejects_zero_and_overflow() {
    let lhs = Value::new(crate::integer::integer16::test_type_id(), 43_i16);
    let rhs = Value::new(crate::integer::integer16::test_type_id(), 5_i16);
    let zero = Value::new(crate::integer::integer16::test_type_id(), 0_i16);
    let minimum = Value::new(crate::integer::integer16::test_type_id(), i16::MIN);
    let negative_one = Value::new(crate::integer::integer16::test_type_id(), -1_i16);

    let result = remainder_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i16>().unwrap(), 3);
    assert!(matches!(
        remainder_integer(&lhs, &zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        remainder_integer(&minimum, &negative_one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed remainder preserves order, output type, zero checks, and overflow.
#[test]
fn calculates_promoted_integer8_remainder_as_integer16() {
    let wider_id = crate::integer::integer16::test_type_id();
    let narrower_id = crate::integer::integer8::test_type_id();
    let wide = Value::new(wider_id, 43_i16);
    let five = Value::new(narrower_id, 5_i8);
    let zero = Value::new(narrower_id, 0_i8);
    let minimum = Value::new(wider_id, i16::MIN);
    let negative_one = Value::new(narrower_id, -1_i8);

    let wide_left = remainder_mixed_integer(&wide, &five).unwrap();
    let narrow_left = remainder_mixed_integer(&five, &wide).unwrap();
    assert_eq!(
        (
            wide_left.type_id(),
            *wide_left.downcast_ref::<i16>().unwrap()
        ),
        (wider_id, 3)
    );
    assert_eq!(
        (
            narrow_left.type_id(),
            *narrow_left.downcast_ref::<i16>().unwrap()
        ),
        (wider_id, 5)
    );
    assert!(matches!(
        remainder_mixed_integer(&wide, &zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        remainder_mixed_integer(&minimum, &negative_one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
    let invalid = Value::new(narrower_id, false);
    assert!(matches!(
        remainder_mixed_integer(&invalid, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}

/// Verifies context-aware remainder promotes the `MIN % -1` overflow to `int32`.
#[test]
fn promotes_overflowed_context_remainder_to_int32() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer16_id = registry.type_by_name("int16").unwrap();
    let int32_id = registry.type_by_name("int32").unwrap();
    let minimum = Value::new(integer16_id, i16::MIN);
    let negative_one = Value::new(integer16_id, -1_i16);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Remainder, integer16_id, integer16_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &minimum, &negative_one).unwrap();

    assert_eq!(promoted.type_id(), int32_id);
    assert_eq!(*promoted.downcast_ref::<i32>().unwrap(), 0);
}
