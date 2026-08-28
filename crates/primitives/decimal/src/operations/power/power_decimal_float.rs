use language_core::{CoreError, Value};

use crate::operations::{checked_power, decimal_exponent, decimal_float_operands};

/// Raises a decimal/float pair to an integer-valued exponent in source order.
pub(crate) fn power_decimal_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, decimal_id) = decimal_float_operands(lhs, rhs)?;
    Ok(Value::new(
        decimal_id,
        checked_power(lhs, decimal_exponent(rhs)?)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    use crate::value::get as get_decimal;

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
}
