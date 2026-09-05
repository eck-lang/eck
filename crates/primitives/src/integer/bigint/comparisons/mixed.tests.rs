use super::*;
use num_bigint::BigInt;

/// Verifies every mixed ordering outcome for a narrower operand.
#[test]
fn compares_promoted_narrower_operands() {
    let wider_id = crate::integer::bigint::test_type_id();
    let narrower_id = crate::integer::integer8::test_type_id();
    let wide = Value::new(wider_id, BigInt::from(7));
    let smaller = Value::new(narrower_id, 4_i8);
    let equal_value = Value::new(narrower_id, 7_i8);
    let larger = Value::new(narrower_id, 10_i8);

    assert!(equal(&wide, &equal_value).unwrap());
    assert!(equal(&equal_value, &wide).unwrap());
    assert!(not_equal(&wide, &smaller).unwrap());
    assert!(not_equal(&smaller, &wide).unwrap());
    assert!(less(&smaller, &wide).unwrap());
    assert!(greater(&wide, &smaller).unwrap());
    assert!(less_or_equal(&equal_value, &wide).unwrap());
    assert!(greater_or_equal(&wide, &equal_value).unwrap());
    assert!(less(&wide, &larger).unwrap());
    assert!(greater(&larger, &wide).unwrap());
}

/// Verifies mixed comparison beyond the 128-bit boundaries.
#[test]
fn compares_promoted_operands_beyond_128_bit_boundaries() {
    let wider_id = crate::integer::bigint::test_type_id();
    let narrower_id = crate::integer::integer128::test_type_id();
    let above_128_max = Value::new(
        wider_id,
        "170141183460469231731687303715884105728"
            .parse::<BigInt>()
            .unwrap(),
    );
    let narrow_maximum = Value::new(narrower_id, i128::MAX);

    assert!(greater(&above_128_max, &narrow_maximum).unwrap());
    assert!(less(&narrow_maximum, &above_128_max).unwrap());
}

/// Verifies that mixed comparison rejects a non-integer runtime representation.
#[test]
fn rejects_non_integer_mixed_payloads() {
    let wider_id = crate::integer::bigint::test_type_id();
    let narrower_id = crate::integer::integer8::test_type_id();
    let wide = Value::new(wider_id, BigInt::from(7));
    let invalid = Value::new(narrower_id, false);

    assert!(matches!(
        equal(&wide, &invalid),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
