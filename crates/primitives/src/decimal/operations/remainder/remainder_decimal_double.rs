use language_core::{CoreError, Value};

use crate::decimal::operations::{checked_remainder, decimal_double_operands};

/// Calculates the remainder of a decimal and a double in source order.
pub(crate) fn remainder_decimal_double(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, decimal_id) =
        decimal_double_operands(left_operand, right_operand)?;
    if right_operand.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(
        decimal_id,
        checked_remainder(left_operand, right_operand)?,
    ))
}

#[cfg(test)]
#[path = "remainder_decimal_double.tests.rs"]
mod tests;
