use language_core::{CoreError, Value};

use crate::operations::double_int_operands;

/// Subtracts a double and an integer while preserving their source order.
pub(crate) fn subtraction_double_int(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, double_id) =
        double_int_operands(left_operand, right_operand)?;
    Ok(Value::new(double_id, left_operand - right_operand))
}

#[cfg(test)]
#[path = "subtraction_double_int.tests.rs"]
mod tests;
