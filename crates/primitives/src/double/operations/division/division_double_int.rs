use language_core::{CoreError, Value};

use crate::double::operations::double_int_operands;

/// Divides a double and an integer while preserving their source order.
pub(crate) fn division_double_int(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, double_id) =
        double_int_operands(left_operand, right_operand)?;
    if right_operand == 0.0 {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(double_id, left_operand / right_operand))
}

#[cfg(test)]
#[path = "division_double_int.tests.rs"]
mod tests;
