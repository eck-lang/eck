use super::*;
use rust_decimal::Decimal;

use crate::value::get as get_decimal;

/// Verifies decimal–float remainder in both operand orders.
#[test]
fn calculates_decimal_float_remainder_in_both_orders() {
    assert_eq!(
        get_decimal(
            &remainder_decimal_float(
                &Value::new(crate::test_type_id(1), Decimal::new(105, 1)),
                &Value::new(crate::test_type_id(2), 2.0_f32),
            )
            .unwrap(),
        )
        .unwrap(),
        Decimal::new(5, 1)
    );
    assert_eq!(
        get_decimal(
            &remainder_decimal_float(
                &Value::new(crate::test_type_id(2), 10.5_f32),
                &Value::new(crate::test_type_id(1), Decimal::new(2, 0)),
            )
            .unwrap(),
        )
        .unwrap(),
        Decimal::new(5, 1)
    );
}
