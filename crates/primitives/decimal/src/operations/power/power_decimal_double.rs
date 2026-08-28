use language_core::{CoreError, Value};

use crate::operations::{checked_power, decimal_double_operands, decimal_exponent};

/// Raises a decimal/double pair to an integer-valued exponent in source order.
pub(crate) fn power_decimal_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, decimal_id) = decimal_double_operands(lhs, rhs)?;
    Ok(Value::new(
        decimal_id,
        checked_power(lhs, decimal_exponent(rhs)?)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::get as get_decimal;
    use rust_decimal::Decimal;

    /// Verifies decimal/double power in both operand orders.
    #[test]
    fn calculates_power_with_double_in_both_orders() {
        let decimal_left = power_decimal_double(
            &Value::new(crate::test_type_id(1), Decimal::new(2, 0)),
            &Value::new(crate::test_type_id(2), 3.0_f64),
        )
        .unwrap();
        let double_left = power_decimal_double(
            &Value::new(crate::test_type_id(2), 2.0_f64),
            &Value::new(crate::test_type_id(1), Decimal::new(3, 0)),
        )
        .unwrap();

        assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(8, 0));
        assert_eq!(get_decimal(&double_left).unwrap(), Decimal::new(8, 0));
    }
}
