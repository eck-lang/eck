use super::*;
use std::str::FromStr;

/// Verifies equality between decimal and double in both operand directions.
#[test]
fn compares_decimal_and_double_in_both_operand_orders() {
    let decimal = Value::new(crate::decimal::test_type_id(0), Decimal::new(25, 1));
    let double = Value::new(crate::decimal::test_type_id(1), 2.5_f64);

    assert!(equal(&decimal, &double).unwrap());
    assert!(equal(&double, &decimal).unwrap());
}

/// Verifies exact comparison at the precision boundaries of both representations.
#[test]
fn preserves_precision_of_decimal_and_double_representations() {
    let decimal = Value::new(
        crate::decimal::test_type_id(0),
        Decimal::from_str("2.000000000000000000000000001").unwrap(),
    );
    let double = Value::new(crate::decimal::test_type_id(1), 2.0_f64);
    let zero = Value::new(crate::decimal::test_type_id(0), Decimal::ZERO);
    let small_double = Value::new(crate::decimal::test_type_id(1), 1e-29_f64);

    assert!(greater(&decimal, &double).unwrap());
    assert!(less(&double, &decimal).unwrap());
    assert!(less(&zero, &small_double).unwrap());
}

/// Verifies that NaN and infinity retain their IEEE-754 comparison behavior.
#[test]
fn preserves_ieee_special_value_semantics() {
    let decimal = Value::new(crate::decimal::test_type_id(0), Decimal::ONE);
    let nan = Value::new(crate::decimal::test_type_id(1), f64::NAN);
    let positive_infinity = Value::new(crate::decimal::test_type_id(1), f64::INFINITY);
    let negative_infinity = Value::new(crate::decimal::test_type_id(1), f64::NEG_INFINITY);

    assert!(!equal(&decimal, &nan).unwrap());
    assert!(not_equal(&decimal, &nan).unwrap());
    assert!(!less(&decimal, &nan).unwrap());
    assert!(!greater(&decimal, &nan).unwrap());
    assert!(less(&decimal, &positive_infinity).unwrap());
    assert!(greater(&decimal, &negative_infinity).unwrap());
}
