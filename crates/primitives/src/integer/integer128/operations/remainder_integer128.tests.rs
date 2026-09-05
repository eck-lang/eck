use super::*;

/// Verifies integer remainder, zero-divisor rejection, and checked overflow.
#[test]
fn calculates_integer_remainder_and_rejects_zero_and_overflow() {
    let lhs = Value::new(crate::integer::integer128::test_type_id(), 43_i128);
    let rhs = Value::new(crate::integer::integer128::test_type_id(), 5_i128);
    let zero = Value::new(crate::integer::integer128::test_type_id(), 0_i128);
    let minimum = Value::new(crate::integer::integer128::test_type_id(), i128::MIN);
    let negative_one = Value::new(crate::integer::integer128::test_type_id(), -1_i128);

    let result = remainder_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<i128>().unwrap(), 3);
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
fn calculates_promoted_integer64_remainder_as_integer128() {
    let wider_id = crate::integer::integer128::test_type_id();
    let narrower_id = crate::integer::integer64::test_type_id();
    let wide = Value::new(wider_id, 43_i128);
    let five = Value::new(narrower_id, 5_i64);
    let zero = Value::new(narrower_id, 0_i64);
    let minimum = Value::new(wider_id, i128::MIN);
    let negative_one = Value::new(narrower_id, -1_i64);

    let wide_left = remainder_mixed_integer(&wide, &five).unwrap();
    let narrow_left = remainder_mixed_integer(&five, &wide).unwrap();
    assert_eq!(
        (
            wide_left.type_id(),
            *wide_left.downcast_ref::<i128>().unwrap()
        ),
        (wider_id, 3)
    );
    assert_eq!(
        (
            narrow_left.type_id(),
            *narrow_left.downcast_ref::<i128>().unwrap()
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
