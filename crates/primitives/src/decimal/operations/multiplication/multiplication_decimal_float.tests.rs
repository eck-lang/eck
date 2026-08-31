use super::*;
use rust_decimal::Decimal;

use crate::decimal::value::get as get_decimal;

/// Verifies decimal–float multiplication in both operand orders.
#[test]
fn multiplies_decimal_and_float_in_both_orders() {
    let decimal = Value::new(crate::decimal::test_type_id(1), Decimal::new(25, 1));
    let float = Value::new(crate::decimal::test_type_id(2), 2.0_f32);

    assert_eq!(
        get_decimal(&multiplication_decimal_float(&decimal, &float).unwrap()).unwrap(),
        Decimal::new(5, 0)
    );
    assert_eq!(
        get_decimal(&multiplication_decimal_float(&float, &decimal).unwrap()).unwrap(),
        Decimal::new(5, 0)
    );
}
