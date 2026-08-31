use language_core::{CoreError, Value};

use crate::decimal::operations::{checked_multiplication, decimal_float_operands};

/// Multiplies one decimal and one single-precision float, regardless of operand order.
pub(crate) fn multiplication_decimal_float(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, decimal_id) =
        decimal_float_operands(left_operand, right_operand)?;
    Ok(Value::new(
        decimal_id,
        checked_multiplication(left_operand, right_operand)?,
    ))
}

#[cfg(test)]
#[path = "multiplication_decimal_float.tests.rs"]
mod tests;
