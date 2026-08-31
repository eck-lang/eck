use super::*;
use crate::decimal::value::get as get_decimal;
use rust_decimal::Decimal;

/// Verifies decimal/double remainder in both operand orders.
#[test]
fn calculates_remainder_with_double_in_both_orders() {
    let decimal_left = remainder_decimal_double(
        &Value::new(crate::decimal::test_type_id(1), Decimal::new(105, 1)),
        &Value::new(crate::decimal::test_type_id(2), 2.0_f64),
    )
    .unwrap();
    let double_left = remainder_decimal_double(
        &Value::new(crate::decimal::test_type_id(2), 10.5_f64),
        &Value::new(crate::decimal::test_type_id(1), Decimal::new(2, 0)),
    )
    .unwrap();

    assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(5, 1));
    assert_eq!(get_decimal(&double_left).unwrap(), Decimal::new(5, 1));
}
