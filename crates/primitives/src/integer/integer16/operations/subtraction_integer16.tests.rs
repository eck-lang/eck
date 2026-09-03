use super::*;

/// Verifies integer subtraction and checked overflow handling.
#[test]
fn subtracts_integers_and_rejects_overflow() {
    let lhs = Value::new(crate::integer::integer16::test_type_id(), 15_i16);
    let rhs = Value::new(crate::integer::integer16::test_type_id(), 27_i16);
    let minimum = Value::new(crate::integer::integer16::test_type_id(), i16::MIN);
    let one = Value::new(crate::integer::integer16::test_type_id(), 1_i16);

    let result = subtraction_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i16>().unwrap(), -12);
    assert!(matches!(
        subtraction_integer(&minimum, &one),
        Err(CoreError::Runtime(message)) if message.contains("overflow")
    ));
}

/// Verifies mixed subtraction preserves order and checked `int16` boundaries.
#[test]
fn subtracts_promoted_integer8_operands_as_integer16() {
    let wider_id = crate::integer::integer16::test_type_id();
    let narrower_id = crate::integer::integer8::test_type_id();
    let minimum = Value::new(wider_id, i16::MIN);
    let negative_one = Value::new(narrower_id, -1_i8);
    let one = Value::new(narrower_id, 1_i8);

    let wide_left = subtraction_mixed_integer(&minimum, &negative_one).unwrap();
    let narrow_left = subtraction_mixed_integer(&negative_one, &minimum).unwrap();
    assert_eq!(wide_left.type_id(), wider_id);
    assert_eq!(*wide_left.downcast_ref::<i16>().unwrap(), i16::MIN + 1);
    assert_eq!(narrow_left.type_id(), wider_id);
    assert_eq!(*narrow_left.downcast_ref::<i16>().unwrap(), i16::MAX);
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
