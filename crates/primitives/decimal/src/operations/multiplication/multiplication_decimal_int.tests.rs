use super::*;
use crate::value::get as get_decimal;
use rust_decimal::Decimal;

/// Verifies decimal/integer multiplication in both operand orders.
#[test]
fn multiplies_decimal_and_integer_in_both_orders() {
    let decimal = Value::new(crate::test_type_id(1), Decimal::new(25, 1));
    let integer = Value::new(crate::test_type_id(2), 2_i64);

    let decimal_left = multiplication_decimal_int(&decimal, &integer).unwrap();
    let integer_left = multiplication_decimal_int(&integer, &decimal).unwrap();

    assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(5, 0));
    assert_eq!(get_decimal(&integer_left).unwrap(), Decimal::new(5, 0));
}
