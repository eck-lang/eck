use super::*;
use num_bigint::BigInt;

/// Builds a `bigint` runtime value from a decimal string.
fn bigint_value(raw_text: &str) -> Value {
    Value::new(
        crate::integer::bigint::test_type_id(),
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
    let wider_id = crate::integer::bigint::test_type_id();
    let narrower_id = crate::integer::integer8::test_type_id();
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
