use language_core::{CoreError, Value};

use crate::decimal::operations::{checked_subtraction, decimal_float_operands};

/// Subtracts a decimal and a single-precision float while preserving source order.
pub(crate) fn subtraction_decimal_float(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, decimal_id) =
        decimal_float_operands(left_operand, right_operand)?;
    Ok(Value::new(
        decimal_id,
        checked_subtraction(left_operand, right_operand)?,
    ))
}

#[cfg(test)]
#[path = "subtraction_decimal_float.tests.rs"]
mod tests;
