use super::*;

use language_core::Registry;
use num_bigint::BigInt;

/// Verifies integer subtraction and checked overflow handling.
#[test]
fn subtracts_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer128::test_type_id(), 15_i128);
    let rhs = Value::new(crate::integer::integer128::test_type_id(), 27_i128);
    let minimum = Value::new(crate::integer::integer128::test_type_id(), i128::MIN);
    let one = Value::new(crate::integer::integer128::test_type_id(), 1_i128);

    let result = subtraction_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i128>().unwrap(), -12);
    assert!(matches!(
        subtraction_integer(&minimum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed subtraction preserves order and checked `int128` boundaries.
#[test]
fn subtracts_promoted_integer64_operands_as_integer128() {
    let wider_id = crate::integer::integer128::test_type_id();
    let narrower_id = crate::integer::integer64::test_type_id();
    let minimum = Value::new(wider_id, i128::MIN);
    let negative_one = Value::new(narrower_id, -1_i64);
    let one = Value::new(narrower_id, 1_i64);

    let wide_left = subtraction_mixed_integer(&minimum, &negative_one).unwrap();
    let narrow_left = subtraction_mixed_integer(&negative_one, &minimum).unwrap();
    assert_eq!(wide_left.type_id(), wider_id);
    assert_eq!(*wide_left.downcast_ref::<i128>().unwrap(), i128::MIN + 1);
    assert_eq!(narrow_left.type_id(), wider_id);
    assert_eq!(*narrow_left.downcast_ref::<i128>().unwrap(), i128::MAX);
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

/// Verifies context-aware subtraction promotes `int128` overflow to `bigint`.
#[test]
fn promotes_overflowed_context_subtraction_to_bigint() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer128_id = registry.type_by_name("int128").unwrap();
    let bigint_id = registry.type_by_name("bigint").unwrap();
    let minimum = Value::new(integer128_id, i128::MIN);
    let one = Value::new(integer128_id, 1_i128);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Subtraction, integer128_id, integer128_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &minimum, &one).unwrap();

    assert_eq!(promoted.type_id(), bigint_id);
    assert_eq!(
        promoted.downcast_ref::<BigInt>().unwrap(),
        &(BigInt::from(i128::MIN) - 1)
    );
}
