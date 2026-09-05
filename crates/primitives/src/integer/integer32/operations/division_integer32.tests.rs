use super::*;

use language_core::Registry;

/// Verifies integer division, zero-divisor rejection, and checked overflow.
#[test]
fn divides_integers_and_rejects_zero_and_overflow() {
    let lhs = Value::new(crate::integer::integer32::test_type_id(), 43_i32);
    let rhs = Value::new(crate::integer::integer32::test_type_id(), 5_i32);
    let zero = Value::new(crate::integer::integer32::test_type_id(), 0_i32);
    let minimum = Value::new(crate::integer::integer32::test_type_id(), i32::MIN);
    let negative_one = Value::new(crate::integer::integer32::test_type_id(), -1_i32);

    let result = division_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i32>().unwrap(), 8);
    assert!(matches!(
        division_integer(&lhs, &zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        division_integer(&minimum, &negative_one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed division preserves order, output type, zero checks, and overflow.
#[test]
fn divides_promoted_integer16_operands_as_integer32() {
    let wider_id = crate::integer::integer32::test_type_id();
    let narrower_id = crate::integer::integer16::test_type_id();
    let wide = Value::new(wider_id, 43_i32);
    let five = Value::new(narrower_id, 5_i16);
    let zero = Value::new(narrower_id, 0_i16);
    let minimum = Value::new(wider_id, i32::MIN);
    let negative_one = Value::new(narrower_id, -1_i16);

    let wide_left = division_mixed_integer(&wide, &five).unwrap();
    let narrow_left = division_mixed_integer(&five, &wide).unwrap();
    assert_eq!(
        (
            wide_left.type_id(),
            *wide_left.downcast_ref::<i32>().unwrap()
        ),
        (wider_id, 8)
    );
    assert_eq!(
        (
            narrow_left.type_id(),
            *narrow_left.downcast_ref::<i32>().unwrap()
        ),
        (wider_id, 0)
    );
    assert!(matches!(
        division_mixed_integer(&wide, &zero),
        Err(CoreError::DivisionByZero)
    ));
    assert!(matches!(
        division_mixed_integer(&minimum, &negative_one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
    let invalid = Value::new(narrower_id, false);
    assert!(matches!(
        division_mixed_integer(&invalid, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}

/// Verifies context-aware division promotes the `MIN / -1` overflow to `int64`.
#[test]
fn promotes_overflowed_context_division_to_int64() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer32_id = registry.type_by_name("int32").unwrap();
    let integer64_id = registry.type_by_name("int64").unwrap();
    let minimum = Value::new(integer32_id, i32::MIN);
    let negative_one = Value::new(integer32_id, -1_i32);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Division, integer32_id, integer32_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &minimum, &negative_one).unwrap();

    assert_eq!(promoted.type_id(), integer64_id);
    assert_eq!(*promoted.downcast_ref::<i64>().unwrap(), 2_147_483_648);
}
