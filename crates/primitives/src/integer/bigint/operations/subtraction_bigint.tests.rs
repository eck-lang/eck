use super::*;
use num_bigint::BigInt;

/// Builds a `bigint` runtime value from a decimal string.
fn bigint_value(raw_text: &str) -> Value {
    Value::new(
        crate::integer::bigint::test_type_id(),
        raw_text.parse::<BigInt>().unwrap(),
    )
}

/// Verifies integer subtraction, including results below the 128-bit minimum.
#[test]
fn subtracts_integers_without_overflow_limit() {
    let lhs = bigint_value("15");
    let rhs = bigint_value("27");
    let below_128_min = bigint_value("-170141183460469231731687303715884105728");
    let one = bigint_value("1");

    let result = subtraction_integer(&lhs, &rhs).unwrap();

    assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), BigInt::from(-12));
    let borrowed = subtraction_integer(&below_128_min, &one).unwrap();
    assert_eq!(
        borrowed.downcast_ref::<BigInt>().unwrap().to_string(),
        "-170141183460469231731687303715884105729"
    );
}

/// Verifies mixed subtraction promotes both orders and keeps the `bigint` result type.
#[test]
fn subtracts_promoted_narrower_operands_as_bigint() {
    let wider_id = crate::integer::bigint::test_type_id();
    let wide = Value::new(wider_id, BigInt::from(100));
    let narrow8 = Value::new(crate::integer::integer8::test_type_id(), 7_i8);
    let narrow128 = Value::new(crate::integer::integer128::test_type_id(), 7_i128);

    for (left_operand, right_operand, expected) in [
        (&wide, &narrow8, BigInt::from(93)),
        (&narrow8, &wide, BigInt::from(-93)),
        (&wide, &narrow128, BigInt::from(93)),
        (&narrow128, &wide, BigInt::from(-93)),
    ] {
        let result = subtraction_mixed_integer(left_operand, right_operand).unwrap();
        assert_eq!(result.type_id(), wider_id);
        assert_eq!(*result.downcast_ref::<BigInt>().unwrap(), expected);
    }
}
