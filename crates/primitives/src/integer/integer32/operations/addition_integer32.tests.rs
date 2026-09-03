use super::*;

/// Verifies integer addition and checked overflow handling.
#[test]
fn adds_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer32::test_type_id(), 15_i32);
    let rhs = Value::new(crate::integer::integer32::test_type_id(), 27_i32);
    let maximum = Value::new(crate::integer::integer32::test_type_id(), i32::MAX);
    let one = Value::new(crate::integer::integer32::test_type_id(), 1_i32);

    let result = addition_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i32>().unwrap(), 42);
    assert!(matches!(
        addition_integer(&maximum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed addition promotes both orders and keeps `int32` boundary errors.
#[test]
fn adds_promoted_integer16_operands_as_integer32() {
    let wider_id = crate::integer::integer32::test_type_id();
    let narrower_id = crate::integer::integer16::test_type_id();
    let maximum = Value::new(wider_id, i32::MAX);
    let negative_one = Value::new(narrower_id, -1_i16);
    let one = Value::new(narrower_id, 1_i16);

    for result in [
        addition_mixed_integer(&maximum, &negative_one).unwrap(),
        addition_mixed_integer(&negative_one, &maximum).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<i32>().unwrap(), i32::MAX - 1);
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
