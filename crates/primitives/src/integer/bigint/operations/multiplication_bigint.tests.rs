use super::*;
use num_bigint::BigInt;

/// Builds a `bigint` runtime value from a decimal string.
fn bigint_value(raw_text: &str) -> Value {
    Value::new(
        crate::integer::bigint::test_type_id(),
        raw_text.parse::<BigInt>().unwrap(),
    )
}

/// Verifies integer multiplication, including products beyond 128 bits.
#[test]
fn multiplies_integers_without_overflow_limit() {
    let lhs = bigint_value("6");
    let rhs = bigint_value("7");
    let large = bigint_value("170141183460469231731687303715884105727");
    let two = bigint_value("2");

    let result = multiplication_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), BigInt::from(42));
    let doubled = multiplication_integer(&large, &two).unwrap();
    assert_eq!(
        doubled.downcast_ref::<BigInt>().unwrap().to_string(),
        "340282366920938463463374607431768211454"
    );
}
