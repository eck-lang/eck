use language_core::{CoreError, Value};

use crate::operations::{checked_power, decimal_exponent, decimal_int_operands};

/// Raises a decimal/int pair to an integer-valued exponent in source order.
pub(crate) fn power_decimal_int(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, decimal_id) = decimal_int_operands(lhs, rhs)?;
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

    /// Verifies decimal/integer power in both operand orders.
    #[test]
    fn calculates_power_with_integer_in_both_orders() {
        let decimal_left = power_decimal_int(
            &Value::new(crate::test_type_id(1), Decimal::new(2, 0)),
            &Value::new(crate::test_type_id(2), 3_i64),
        )
        .unwrap();
        let integer_left = power_decimal_int(
            &Value::new(crate::test_type_id(2), 2_i64),
            &Value::new(crate::test_type_id(1), Decimal::new(3, 0)),
        )
        .unwrap();

        assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(8, 0));
        assert_eq!(get_decimal(&integer_left).unwrap(), Decimal::new(8, 0));
    }
}
