use language_core::{CoreError, Value};

use crate::decimal::operations::{checked_subtraction, decimal_int_operands};

/// Subtracts a decimal and an integer while preserving their source order.
pub(crate) fn subtraction_decimal_int(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, decimal_id) =
        decimal_int_operands(left_operand, right_operand)?;
    Ok(Value::new(
        decimal_id,
        checked_subtraction(left_operand, right_operand)?,
    ))
}

#[cfg(test)]
#[path = "subtraction_decimal_int.tests.rs"]
mod tests;
