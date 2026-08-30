use super::*;
use rust_decimal::Decimal;

use crate::value::get as get_decimal;

/// Verifies decimal/integer subtraction in both operand orders.
#[test]
fn subtracts_decimal_and_integer_in_both_orders() {
    let decimal = Value::new(crate::test_type_id(1), Decimal::new(25, 1));
    let integer = Value::new(crate::test_type_id(2), 2_i64);

    let decimal_left = subtraction_decimal_int(&decimal, &integer).unwrap();
    let integer_left = subtraction_decimal_int(&integer, &decimal).unwrap();

    assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(5, 1));
    assert_eq!(get_decimal(&integer_left).unwrap(), Decimal::new(-5, 1));
}
