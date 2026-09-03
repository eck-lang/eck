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
