use super::*;

/// Verifies integer addition and checked overflow handling.
#[test]
fn adds_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer128::test_type_id(), 15_i128);
    let rhs = Value::new(crate::integer::integer128::test_type_id(), 27_i128);
    let maximum = Value::new(crate::integer::integer128::test_type_id(), i128::MAX);
    let one = Value::new(crate::integer::integer128::test_type_id(), 1_i128);

    let result = addition_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i128>().unwrap(), 42);
    assert!(matches!(
        addition_integer(&maximum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed addition promotes both orders and keeps `int128` boundary errors.
#[test]
fn adds_promoted_integer64_operands_as_integer128() {
    let wider_id = crate::integer::integer128::test_type_id();
    let narrower_id = crate::integer::integer64::test_type_id();
    let maximum = Value::new(wider_id, i128::MAX);
    let negative_one = Value::new(narrower_id, -1_i64);
    let one = Value::new(narrower_id, 1_i64);

    for result in [
        addition_mixed_integer(&maximum, &negative_one).unwrap(),
        addition_mixed_integer(&negative_one, &maximum).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<i128>().unwrap(), i128::MAX - 1);
    }
    assert!(matches!(
        addition_mixed_integer(&maximum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
    let invalid = Value::new(narrower_id, false);
    assert!(matches!(
        addition_mixed_integer(&invalid, &maximum),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
