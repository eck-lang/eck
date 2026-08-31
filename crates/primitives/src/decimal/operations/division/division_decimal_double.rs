use language_core::{CoreError, Value};

use crate::decimal::operations::{checked_division, decimal_double_operands};

/// Divides a decimal and a double while preserving their source order.
pub(crate) fn division_decimal_double(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, decimal_id) =
        decimal_double_operands(left_operand, right_operand)?;
    if right_operand.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(
        decimal_id,
        checked_division(left_operand, right_operand)?,
    ))
}

#[cfg(test)]
#[path = "division_decimal_double.tests.rs"]
mod tests;
