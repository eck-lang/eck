use super::*;
use num_bigint::BigInt;

/// Builds a `bigint` runtime value from a decimal string.
fn bigint_value(raw_text: &str) -> Value {
    Value::new(
        crate::integer::bigint::test_type_id(),
        raw_text.parse::<BigInt>().unwrap(),
    )
}

/// Verifies integer division and zero-divisor rejection.
#[test]
fn divides_integers_and_rejects_zero_divisors() {
    let lhs = bigint_value("43");
    let rhs = bigint_value("5");
    let zero = bigint_value("0");
    let negative_dividend = bigint_value("-43");

    let result = division_integer(&lhs, &rhs).unwrap();
    let truncated = division_integer(&negative_dividend, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), BigInt::from(8));
    assert_eq!(
        *truncated.downcast_ref::<BigInt>().unwrap(),
        BigInt::from(-8)
    );
    assert!(matches!(
        division_integer(&lhs, &zero),
        Err(CoreError::DivisionByZero)
    ));
}

/// Verifies mixed division promotes both orders and rejects zero divisors.
#[test]
fn divides_promoted_narrower_operands_as_bigint() {
    let wider_id = crate::integer::bigint::test_type_id();
    let narrower_id = crate::integer::integer8::test_type_id();
    let wide = Value::new(wider_id, BigInt::from(43));
    let narrow = Value::new(narrower_id, 5_i8);
    let zero = Value::new(narrower_id, 0_i8);

    for (left_operand, right_operand, expected) in [
        (&wide, &narrow, BigInt::from(8)),
        (&narrow, &wide, BigInt::from(0)),
    ] {
        let result = division_mixed_integer(left_operand, right_operand).unwrap();
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), expected);
    }
    assert!(matches!(
        division_mixed_integer(&wide, &zero),
        Err(CoreError::DivisionByZero)
    ));
}
