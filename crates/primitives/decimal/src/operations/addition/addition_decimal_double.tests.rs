use super::*;
use rust_decimal::Decimal;

use crate::value::get as get_decimal;

/// Verifies decimal/double addition in both operand orders.
#[test]
fn adds_decimal_and_double_in_both_orders() {
    let decimal = Value::new(crate::test_type_id(1), Decimal::new(25, 1));
    let double = Value::new(crate::test_type_id(2), 2.0_f64);

    let decimal_left = addition_decimal_double(&decimal, &double).unwrap();
    let double_left = addition_decimal_double(&double, &decimal).unwrap();

    assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(45, 1));
    assert_eq!(get_decimal(&double_left).unwrap(), Decimal::new(45, 1));
}
