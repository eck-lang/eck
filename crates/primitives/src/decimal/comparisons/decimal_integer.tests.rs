use super::*;
use std::str::FromStr;

/// Verifies decimal–integer ordering in both operand directions.
#[test]
fn compares_decimal_and_integer_in_both_operand_orders() {
    let decimal = Value::new(crate::decimal::test_type_id(0), Decimal::new(25, 1));
    let integer = Value::new(crate::decimal::test_type_id(1), 2_i64);

    assert!(greater(&decimal, &integer).unwrap());
    assert!(less(&integer, &decimal).unwrap());
}

/// Verifies that integer promotion preserves decimal fractional precision.
#[test]
fn converts_integer_to_decimal_without_losing_precision() {
    let decimal = Value::new(
        crate::decimal::test_type_id(0),
        Decimal::from_str("2.000000000000000000000000001").unwrap(),
    );
    let integer = Value::new(crate::decimal::test_type_id(1), 2_i64);

    assert!(greater(&decimal, &integer).unwrap());
    assert!(!equal(&decimal, &integer).unwrap());
}
