use language_core::{CoreError, Value};

use crate::float::operations::float_int_operands;

/// Divides a float and an integer while preserving their source order.
pub(crate) fn division_float_int(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, float_id) = float_int_operands(left_operand, right_operand)?;
    if right_operand == 0.0 {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(float_id, left_operand / right_operand))
}

#[cfg(test)]
#[path = "division_float_int.tests.rs"]
mod tests;
