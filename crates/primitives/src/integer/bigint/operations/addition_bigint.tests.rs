use super::*;
use num_bigint::BigInt;

/// Builds a `bigint` runtime value from a decimal string.
fn bigint_value(raw_text: &str) -> Value {
    Value::new(
        crate::integer::bigint::test_type_id(),
        raw_text.parse::<BigInt>().unwrap(),
    )
}

/// Verifies integer addition, including magnitudes beyond 128 bits.
#[test]
fn adds_integers_without_overflow_limit() {
    let lhs = bigint_value("15");
    let rhs = bigint_value("27");
    let above_128_max = bigint_value("170141183460469231731687303715884105727");
    let one = bigint_value("1");

    let result = addition_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), BigInt::from(42));
    let carried = addition_integer(&above_128_max, &one).unwrap();
    assert_eq!(
        carried.downcast_ref::<BigInt>().unwrap().to_string(),
        "170141183460469231731687303715884105728"
    );
}

/// Verifies mixed addition promotes both orders and keeps the `bigint` result type.
#[test]
fn adds_promoted_narrower_operands_as_bigint() {
    let wider_id = crate::integer::bigint::test_type_id();
    let wide = Value::new(wider_id, BigInt::from(100));
    let narrow8 = Value::new(crate::integer::integer8::test_type_id(), 7_i8);
    let narrow128 = Value::new(crate::integer::integer128::test_type_id(), 7_i128);

    for result in [
        addition_mixed_integer(&wide, &narrow8).unwrap(),
        addition_mixed_integer(&narrow8, &wide).unwrap(),
        addition_mixed_integer(&wide, &narrow128).unwrap(),
        addition_mixed_integer(&narrow128, &wide).unwrap(),
    ] {
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), BigInt::from(107));
    }
    let invalid = Value::new(crate::integer::integer8::test_type_id(), false);
    assert!(matches!(
        addition_mixed_integer(&invalid, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
