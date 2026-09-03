use super::*;

/// Verifies integer division, zero-divisor rejection, and checked overflow.
#[test]
fn divides_integers_and_rejects_zero_and_overflow() {
    let lhs = Value::new(crate::integer::integer16::test_type_id(), 43_i16);
    let rhs = Value::new(crate::integer::integer16::test_type_id(), 5_i16);
    let zero = Value::new(crate::integer::integer16::test_type_id(), 0_i16);
    let minimum = Value::new(crate::integer::integer16::test_type_id(), i16::MIN);
    let negative_one = Value::new(crate::integer::integer16::test_type_id(), -1_i16);

    let result = division_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i16>().unwrap(), 8);
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
fn divides_promoted_integer8_operands_as_integer16() {
    let wider_id = crate::integer::integer16::test_type_id();
    let narrower_id = crate::integer::integer8::test_type_id();
    let wide = Value::new(wider_id, 43_i16);
    let five = Value::new(narrower_id, 5_i8);
    let zero = Value::new(narrower_id, 0_i8);
    let minimum = Value::new(wider_id, i16::MIN);
    let negative_one = Value::new(narrower_id, -1_i8);

    let wide_left = division_mixed_integer(&wide, &five).unwrap();
    let narrow_left = division_mixed_integer(&five, &wide).unwrap();
    assert_eq!(
        (
            wide_left.type_id(),
            *wide_left.downcast_ref::<i16>().unwrap()
        ),
        (wider_id, 8)
    );
    assert_eq!(
        (
            narrow_left.type_id(),
            *narrow_left.downcast_ref::<i16>().unwrap()
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
