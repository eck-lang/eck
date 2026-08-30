use language_core::{CoreError, Value};

use crate::operations::double_int_operands;

/// Raises a double/int pair to a power after converting the integer to double precision.
pub(crate) fn power_double_int(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, double_id) =
        double_int_operands(left_operand, right_operand)?;
    Ok(Value::new(double_id, left_operand.powf(right_operand)))
}

#[cfg(test)]
#[path = "power_double_int.tests.rs"]
mod tests;
