use language_core::{CoreError, Value};

use crate::operations::double_int_operands;

/// Calculates the remainder of a double and an integer in source order.
pub(crate) fn remainder_double_int(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, double_id) =
        double_int_operands(left_operand, right_operand)?;
    if right_operand == 0.0 {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(double_id, left_operand % right_operand))
}

#[cfg(test)]
#[path = "remainder_double_int.tests.rs"]
mod tests;
