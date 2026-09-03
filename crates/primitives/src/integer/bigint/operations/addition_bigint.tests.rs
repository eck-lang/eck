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
