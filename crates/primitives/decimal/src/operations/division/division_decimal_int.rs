use language_core::{CoreError, Value};

use crate::operations::{checked_division, decimal_int_operands};

/// Divides a decimal and an integer while preserving their source order.
pub(crate) fn division_decimal_int(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, decimal_id) =
        decimal_int_operands(left_operand, right_operand)?;
    if right_operand.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(
        decimal_id,
        checked_division(left_operand, right_operand)?,
    ))
}

#[cfg(test)]
#[path = "division_decimal_int.tests.rs"]
mod tests;
