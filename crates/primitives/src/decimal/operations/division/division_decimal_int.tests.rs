use super::*;
use crate::decimal::value::get as get_decimal;
use rust_decimal::Decimal;

/// Verifies decimal/integer division in both operand orders.
#[test]
fn divides_decimal_and_integer_in_both_orders() {
    let decimal_left = division_decimal_int(
        &Value::new(crate::decimal::test_type_id(1), Decimal::new(5, 0)),
        &Value::new(crate::decimal::test_type_id(2), 2_i64),
    )
    .unwrap();
    let integer_left = division_decimal_int(
        &Value::new(crate::decimal::test_type_id(2), 5_i64),
        &Value::new(crate::decimal::test_type_id(1), Decimal::new(2, 0)),
    )
    .unwrap();

    assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(25, 1));
    assert_eq!(get_decimal(&integer_left).unwrap(), Decimal::new(25, 1));
}
