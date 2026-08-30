use super::*;
use rust_decimal::Decimal;

use crate::value::get as get_decimal;

/// Verifies decimal–float exponentiation in both operand orders.
#[test]
fn calculates_decimal_float_power_in_both_orders() {
    assert_eq!(
        get_decimal(
            &power_decimal_float(
                &Value::new(crate::test_type_id(1), Decimal::new(2, 0)),
                &Value::new(crate::test_type_id(2), 3.0_f32),
            )
            .unwrap(),
        )
        .unwrap(),
        Decimal::new(8, 0)
    );
    assert_eq!(
        get_decimal(
            &power_decimal_float(
                &Value::new(crate::test_type_id(2), 2.0_f32),
                &Value::new(crate::test_type_id(1), Decimal::new(3, 0)),
            )
            .unwrap(),
        )
        .unwrap(),
        Decimal::new(8, 0)
    );
}
