use super::*;

use language_core::Registry;
use num_bigint::BigInt;

/// Verifies integer division, zero-divisor rejection, and checked overflow.
#[test]
fn divides_integers_and_rejects_zero_and_overflow() {
    let lhs = Value::new(crate::integer::integer128::test_type_id(), 43_i128);
    let rhs = Value::new(crate::integer::integer128::test_type_id(), 5_i128);
    let zero = Value::new(crate::integer::integer128::test_type_id(), 0_i128);
    let minimum = Value::new(crate::integer::integer128::test_type_id(), i128::MIN);
    let negative_one = Value::new(crate::integer::integer128::test_type_id(), -1_i128);

    let result = division_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i128>().unwrap(), 8);
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
fn divides_promoted_integer64_operands_as_integer128() {
    let wider_id = crate::integer::integer128::test_type_id();
    let narrower_id = crate::integer::integer64::test_type_id();
    let wide = Value::new(wider_id, 43_i128);
    let five = Value::new(narrower_id, 5_i64);
    let zero = Value::new(narrower_id, 0_i64);
    let minimum = Value::new(wider_id, i128::MIN);
    let negative_one = Value::new(narrower_id, -1_i64);

    let wide_left = division_mixed_integer(&wide, &five).unwrap();
    let narrow_left = division_mixed_integer(&five, &wide).unwrap();
    assert_eq!(
        (
            wide_left.type_id(),
            *wide_left.downcast_ref::<i128>().unwrap()
        ),
        (wider_id, 8)
    );
    assert_eq!(
        (
            narrow_left.type_id(),
            *narrow_left.downcast_ref::<i128>().unwrap()
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

/// Verifies context-aware division promotes the `MIN / -1` overflow to `bigint`.
#[test]
fn promotes_overflowed_context_division_to_bigint() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer128_id = registry.type_by_name("int128").unwrap();
    let bigint_id = registry.type_by_name("bigint").unwrap();
    let minimum = Value::new(integer128_id, i128::MIN);
    let negative_one = Value::new(integer128_id, -1_i128);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Division, integer128_id, integer128_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &minimum, &negative_one).unwrap();

    assert_eq!(promoted.type_id(), bigint_id);
    assert_eq!(
        promoted.downcast_ref::<BigInt>().unwrap(),
        &(BigInt::from(i128::MIN) / -1)
    );
}
