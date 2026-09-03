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
