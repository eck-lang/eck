use language_core::{CoreError, Value};

use crate::operations::double_int_operands;

/// Multiplies a double and an integer after converting the integer to double precision.
pub(crate) fn multiplication_double_int(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, double_id) =
        double_int_operands(left_operand, right_operand)?;
    Ok(Value::new(double_id, left_operand * right_operand))
}

#[cfg(test)]
#[path = "multiplication_double_int.tests.rs"]
mod tests;
