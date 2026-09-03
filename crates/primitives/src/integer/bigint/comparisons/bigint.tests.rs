use super::*;
use num_bigint::BigInt;

/// Verifies all ordering outcomes for ordinary integer values.
#[test]
fn compares_integer_payloads() {
    let smaller = Value::new(crate::integer::bigint::test_type_id(), BigInt::from(4));
    let equal_value = Value::new(crate::integer::bigint::test_type_id(), BigInt::from(4));
    let larger = Value::new(crate::integer::bigint::test_type_id(), BigInt::from(7));

    assert!(equal(&smaller, &equal_value).unwrap());
    assert!(!not_equal(&smaller, &equal_value).unwrap());
    assert!(less(&smaller, &larger).unwrap());
    assert!(less_or_equal(&smaller, &equal_value).unwrap());
    assert!(greater(&larger, &smaller).unwrap());
    assert!(greater_or_equal(&larger, &smaller).unwrap());
}

/// Verifies total ordering for magnitudes beyond the 128-bit boundaries.
#[test]
fn compares_values_beyond_128_bit_boundaries() {
    let below_128_min = Value::new(
        crate::integer::bigint::test_type_id(),
        "-170141183460469231731687303715884105729"
            .parse::<BigInt>()
            .unwrap(),
    );
    let above_128_max = Value::new(
        crate::integer::bigint::test_type_id(),
        "170141183460469231731687303715884105728"
            .parse::<BigInt>()
            .unwrap(),
    );

    assert!(less(&below_128_min, &above_128_max).unwrap());
    assert!(greater(&above_128_max, &below_128_min).unwrap());
}

/// Verifies that comparison rejects a non-integer runtime representation.
#[test]
fn rejects_non_integer_payloads() {
    let integer = Value::new(crate::integer::bigint::test_type_id(), BigInt::from(1));
    let float = Value::new(crate::integer::bigint::test_type_id(), 1.0_f32);

    assert!(matches!(
        equal(&integer, &float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "bigint"
    ));
}
