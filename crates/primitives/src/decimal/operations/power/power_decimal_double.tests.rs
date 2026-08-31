use super::*;
use crate::decimal::value::get as get_decimal;
use rust_decimal::Decimal;

/// Verifies decimal/double power in both operand orders.
#[test]
fn calculates_power_with_double_in_both_orders() {
    let decimal_left = power_decimal_double(
        &Value::new(crate::decimal::test_type_id(1), Decimal::new(2, 0)),
        &Value::new(crate::decimal::test_type_id(2), 3.0_f64),
    )
    .unwrap();
    let double_left = power_decimal_double(
        &Value::new(crate::decimal::test_type_id(2), 2.0_f64),
        &Value::new(crate::decimal::test_type_id(1), Decimal::new(3, 0)),
    )
    .unwrap();

    assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(8, 0));
    assert_eq!(get_decimal(&double_left).unwrap(), Decimal::new(8, 0));
}
