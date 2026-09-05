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

/// Verifies mixed multiplication promotes both orders and keeps the `bigint` result type.
#[test]
fn multiplies_promoted_narrower_operands_as_bigint() {
    let wider_id = crate::integer::bigint::test_type_id();
    let wide = Value::new(wider_id, BigInt::from(100));
    let narrow8 = Value::new(crate::integer::integer8::test_type_id(), 7_i8);
    let narrow128 = Value::new(crate::integer::integer128::test_type_id(), 7_i128);

    for result in [
        multiplication_mixed_integer(&wide, &narrow8).unwrap(),
        multiplication_mixed_integer(&narrow8, &wide).unwrap(),
        multiplication_mixed_integer(&wide, &narrow128).unwrap(),
        multiplication_mixed_integer(&narrow128, &wide).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), BigInt::from(700));
    }
}
