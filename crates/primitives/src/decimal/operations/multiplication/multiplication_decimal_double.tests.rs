use super::*;
use crate::decimal::value::get as get_decimal;
use rust_decimal::Decimal;

/// Verifies decimal/double multiplication in both operand orders.
#[test]
fn multiplies_decimal_and_double_in_both_orders() {
    let decimal = Value::new(crate::decimal::test_type_id(1), Decimal::new(25, 1));
    let double = Value::new(crate::decimal::test_type_id(2), 2.0_f64);

    let decimal_left = multiplication_decimal_double(&decimal, &double).unwrap();
    let double_left = multiplication_decimal_double(&double, &decimal).unwrap();

    assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(5, 0));
    assert_eq!(get_decimal(&double_left).unwrap(), Decimal::new(5, 0));
}
