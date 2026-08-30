use language_core::{CoreError, Value};

use crate::operations::float_int_operands;

/// Subtracts a float and an integer while preserving their source order.
pub(crate) fn subtraction_float_int(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, float_id) = float_int_operands(left_operand, right_operand)?;
    Ok(Value::new(float_id, left_operand - right_operand))
}

#[cfg(test)]
#[path = "subtraction_float_int.tests.rs"]
mod tests;
