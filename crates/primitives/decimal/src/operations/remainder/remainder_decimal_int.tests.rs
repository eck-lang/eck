use super::*;
use crate::value::get as get_decimal;
use rust_decimal::Decimal;

/// Verifies decimal/integer remainder in both operand orders.
#[test]
fn calculates_remainder_with_integer_in_both_orders() {
    let decimal_left = remainder_decimal_int(
        &Value::new(crate::test_type_id(1), Decimal::new(105, 1)),
        &Value::new(crate::test_type_id(2), 2_i64),
    )
    .unwrap();
    let integer_left = remainder_decimal_int(
        &Value::new(crate::test_type_id(2), 10_i64),
        &Value::new(crate::test_type_id(1), Decimal::new(3, 0)),
    )
    .unwrap();

    assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(5, 1));
    assert_eq!(get_decimal(&integer_left).unwrap(), Decimal::new(1, 0));
}
