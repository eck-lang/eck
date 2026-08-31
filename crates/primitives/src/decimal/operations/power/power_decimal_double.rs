use language_core::{CoreError, Value};

use crate::decimal::operations::{checked_power, decimal_double_operands, decimal_exponent};

/// Raises a decimal/double pair to an integer-valued exponent in source order.
pub(crate) fn power_decimal_double(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, decimal_id) =
        decimal_double_operands(left_operand, right_operand)?;
    Ok(Value::new(
        decimal_id,
        checked_power(left_operand, decimal_exponent(right_operand)?)?,
    ))
}

#[cfg(test)]
#[path = "power_decimal_double.tests.rs"]
mod tests;
