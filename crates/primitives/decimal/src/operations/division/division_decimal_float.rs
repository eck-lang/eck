use language_core::{CoreError, Value};

use crate::operations::{checked_division, decimal_float_operands};

/// Divides a decimal and a single-precision float while preserving source order.
pub(crate) fn division_decimal_float(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, decimal_id) =
        decimal_float_operands(left_operand, right_operand)?;
    if right_operand.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(
        decimal_id,
        checked_division(left_operand, right_operand)?,
    ))
}

#[cfg(test)]
#[path = "division_decimal_float.tests.rs"]
mod tests;
