use super::*;
use num_bigint::BigInt;

/// Builds a `bigint` runtime value from a decimal string.
fn bigint_value(raw_text: &str) -> Value {
    Value::new(
        crate::primitives::integer::bigint::test_type_id(),
        raw_text.parse::<BigInt>().unwrap(),
    )
}

/// Verifies the integer remainder and zero-divisor rejection.
#[test]
fn calculates_integer_remainder_and_rejects_zero_divisors() {
    let lhs = bigint_value("43");
    let rhs = bigint_value("5");
    let zero = bigint_value("0");

    let result = remainder_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), BigInt::from(3));
    assert!(matches!(
        remainder_integer(&lhs, &zero),
        Err(CoreError::DivisionByZero)
    ));
}

/// Verifies mixed remainder promotes both orders and rejects zero divisors.
#[test]
fn calculates_promoted_narrower_remainder_as_bigint() {
    let wider_id = crate::primitives::integer::bigint::test_type_id();
    let narrower_id = crate::primitives::integer::integer8::test_type_id();
    let wide = Value::new(wider_id, BigInt::from(43));
    let narrow = Value::new(narrower_id, 5_i8);
    let zero = Value::new(narrower_id, 0_i8);

    for (left_operand, right_operand, expected) in [
        (&wide, &narrow, BigInt::from(3)),
        (&narrow, &wide, BigInt::from(5)),
    ] {
        let result = remainder_mixed_integer(left_operand, right_operand).unwrap();
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), expected);
    }
    assert!(matches!(
        remainder_mixed_integer(&wide, &zero),
        Err(CoreError::DivisionByZero)
    ));
}

/// Verifies a positive power-of-two divisor reduces through the mask path.
#[test]
fn masks_power_of_two_divisors_in_both_remainder_forms() {
    let cases = [
        ("43", "16", "11"),
        ("-43", "16", "-11"),
        ("15", "16", "15"),
        ("-15", "16", "-15"),
        ("16", "16", "0"),
        ("0", "16", "0"),
        ("1", "1", "0"),
    ];
    for (dividend, divisor, expected) in cases {
        let lhs = bigint_value(dividend);
        let rhs = bigint_value(divisor);
        let expected = expected.parse::<BigInt>().unwrap();

        // The in-place form requires a value that owns its payload exclusively,
        // so build one instead of cloning the shared operand.
        let mut owned = bigint_value(dividend);
        remainder_integer_in_place(&mut owned, &rhs).unwrap();
        assert_eq!(
            *owned.downcast_ref::<BigInt>().unwrap(),
            expected,
            "in-place {dividend} % {divisor}"
        );

        let result = remainder_integer(&lhs, &rhs).unwrap();
        assert_eq!(
            *result.downcast_ref::<BigInt>().unwrap(),
            expected,
            "{dividend} % {divisor}"
        );
    }
}

/// Verifies bits above the exponent, including a partial top digit, are dropped.
#[test]
fn masks_bits_above_the_divisor_exponent() {
    let divisor = BigInt::from(1) << 100_u32;
    let dividend = (BigInt::from(1) << 130_u32) + BigInt::from(7);
    let expected = BigInt::from(7);
    let type_id = crate::primitives::integer::bigint::test_type_id();
    let lhs = Value::new(type_id, dividend.clone());
    let rhs = Value::new(type_id, divisor);

    let result = remainder_integer(&lhs, &rhs).unwrap();
    assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), expected);

    let mut owned = Value::new(type_id, dividend);
    remainder_integer_in_place(&mut owned, &rhs).unwrap();
    assert_eq!(*owned.downcast_ref::<BigInt>().unwrap(), expected);
}

/// Verifies other divisors keep the division path with unchanged semantics.
#[test]
fn keeps_the_division_path_for_other_divisors() {
    let cases = [
        ("43", "5", "3"),
        ("-43", "5", "-3"),
        ("43", "-16", "11"),
        ("-43", "-16", "-11"),
        ("123456789012345678901234567890", "7", "0"),
    ];
    for (dividend, divisor, expected) in cases {
        let lhs = bigint_value(dividend);
        let rhs = bigint_value(divisor);
        let expected = expected.parse::<BigInt>().unwrap();

        // The in-place form needs a value that owns its payload exclusively.
        let mut owned = bigint_value(dividend);
        remainder_integer_in_place(&mut owned, &rhs).unwrap();
        assert_eq!(
            *owned.downcast_ref::<BigInt>().unwrap(),
            expected,
            "in-place {dividend} % {divisor}"
        );

        let result = remainder_integer(&lhs, &rhs).unwrap();
        assert_eq!(
            *result.downcast_ref::<BigInt>().unwrap(),
            expected,
            "{dividend} % {divisor}"
        );
    }
}

/// Verifies a divisor wider than the mask buffer still reduces correctly.
#[test]
fn falls_back_when_the_divisor_exceeds_the_mask_buffer() {
    let divisor = BigInt::from(1) << 3000_u32;
    let dividend = divisor.clone() + BigInt::from(5);
    let type_id = crate::primitives::integer::bigint::test_type_id();
    let lhs = Value::new(type_id, dividend.clone());
    let rhs = Value::new(type_id, divisor);
    let expected = BigInt::from(5);

    let result = remainder_integer(&lhs, &rhs).unwrap();
    assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), expected);

    let mut owned = Value::new(type_id, dividend);
    remainder_integer_in_place(&mut owned, &rhs).unwrap();
    assert_eq!(*owned.downcast_ref::<BigInt>().unwrap(), expected);
}
