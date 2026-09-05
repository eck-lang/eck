use super::*;

use language_core::Registry;
use num_bigint::BigInt;

/// Verifies integer multiplication and checked overflow handling.
#[test]
fn multiplies_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer128::test_type_id(), 6_i128);
    let rhs = Value::new(crate::integer::integer128::test_type_id(), 7_i128);
    let maximum = Value::new(crate::integer::integer128::test_type_id(), i128::MAX);
    let two = Value::new(crate::integer::integer128::test_type_id(), 2_i128);

    let result = multiplication_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i128>().unwrap(), 42);
    assert!(matches!(
        multiplication_integer(&maximum, &two),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed multiplication promotes both orders and checks `int128` overflow.
#[test]
fn multiplies_promoted_integer64_operands_as_integer128() {
    let wider_id = crate::integer::integer128::test_type_id();
    let narrower_id = crate::integer::integer64::test_type_id();
    let maximum = Value::new(wider_id, i128::MAX);
    let one = Value::new(narrower_id, 1_i64);
    let two = Value::new(narrower_id, 2_i64);

    for result in [
        multiplication_mixed_integer(&maximum, &one).unwrap(),
        multiplication_mixed_integer(&one, &maximum).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<i128>().unwrap(), i128::MAX);
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

/// Verifies context-aware multiplication promotes `int128` overflow to `bigint`.
#[test]
fn promotes_overflowed_context_multiplication_to_bigint() {
    let mut registry = Registry::new();
    crate::register_all(&mut registry).unwrap();
    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let integer128_id = registry.type_by_name("int128").unwrap();
    let bigint_id = registry.type_by_name("bigint").unwrap();
    let maximum = Value::new(integer128_id, i128::MAX);
    let two = Value::new(integer128_id, 2_i128);
    let operator = registry
        .resolve_binary_operator(BinaryOperator::Multiplication, integer128_id, integer128_id)
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    let promoted = descriptor.context_execute.unwrap()(&context, &maximum, &two).unwrap();

    assert_eq!(promoted.type_id(), bigint_id);
    assert_eq!(
        promoted.downcast_ref::<BigInt>().unwrap(),
        &(BigInt::from(i128::MAX) * 2)
    );
}
