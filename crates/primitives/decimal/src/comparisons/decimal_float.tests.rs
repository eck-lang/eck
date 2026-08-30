use super::*;
use std::str::FromStr;

/// Verifies equality between decimal and float in both operand directions.
#[test]
fn compares_decimal_and_float_in_both_operand_orders() {
    let decimal = Value::new(crate::test_type_id(0), Decimal::new(25, 1));
    let float = Value::new(crate::test_type_id(1), 2.5_f32);

    assert!(equal(&decimal, &float).unwrap());
    assert!(equal(&float, &decimal).unwrap());
}

/// Verifies exact comparison at the precision boundaries of both representations.
#[test]
fn preserves_precision_of_decimal_and_float_representations() {
    let decimal = Value::new(
        crate::test_type_id(0),
        Decimal::from_str("2.000000000000000000000000001").unwrap(),
    );
    let float = Value::new(crate::test_type_id(1), 2.0_f32);
    let zero = Value::new(crate::test_type_id(0), Decimal::ZERO);
    let small_float = Value::new(crate::test_type_id(1), f32::MIN_POSITIVE);

    assert!(greater(&decimal, &float).unwrap());
    assert!(less(&float, &decimal).unwrap());
    assert!(less(&zero, &small_float).unwrap());
}

/// Verifies that NaN and infinity retain their IEEE-754 comparison behavior.
#[test]
fn preserves_ieee_special_value_semantics() {
    let decimal = Value::new(crate::test_type_id(0), Decimal::ONE);
    let nan = Value::new(crate::test_type_id(1), f32::NAN);
    let positive_infinity = Value::new(crate::test_type_id(1), f32::INFINITY);

    assert!(!equal(&decimal, &nan).unwrap());
    assert!(not_equal(&decimal, &nan).unwrap());
    assert!(!less(&decimal, &nan).unwrap());
    assert!(!greater(&decimal, &nan).unwrap());
    assert!(less(&decimal, &positive_infinity).unwrap());
}
