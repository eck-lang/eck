use language_core::{CoreError, Value};

use crate::float::operations::float_int_operands;

/// Raises a float/int pair to a power after converting the integer to single precision.
pub(crate) fn power_float_int(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, float_id) = float_int_operands(left_operand, right_operand)?;
    Ok(Value::new(float_id, left_operand.powf(right_operand)))
}

#[cfg(test)]
#[path = "power_float_int.tests.rs"]
mod tests;
