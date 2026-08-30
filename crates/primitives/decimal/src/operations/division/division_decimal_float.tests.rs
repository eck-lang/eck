use super::*;
use rust_decimal::Decimal;

use crate::value::get as get_decimal;

/// Verifies decimal–float division in both operand orders.
#[test]
fn divides_decimal_and_float_in_both_orders() {
    let decimal = Value::new(crate::test_type_id(1), Decimal::new(5, 0));
    let float = Value::new(crate::test_type_id(2), 2.0_f32);

    assert_eq!(
        get_decimal(&division_decimal_float(&decimal, &float).unwrap()).unwrap(),
        Decimal::new(25, 1)
    );
    assert_eq!(
        get_decimal(
            &division_decimal_float(
                &Value::new(crate::test_type_id(2), 5.0_f32),
                &Value::new(crate::test_type_id(1), Decimal::new(2, 0)),
            )
            .unwrap(),
        )
        .unwrap(),
        Decimal::new(25, 1)
    );
}
