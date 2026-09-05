use language_core::{CoreError, Value};

use crate::integer::integer128::value::{get, mixed_operands};

/// Subtracts two integers and reports overflow as a language error.
pub(crate) fn subtraction_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)?
        .checked_sub(get(rhs)?)
        .ok_or_else(|| CoreError::Runtime("integer overflow in subtraction".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Subtracts mixed-width integers after losslessly promoting both operands to `int128`.
pub(crate) fn subtraction_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    let value = left_operand
        .checked_sub(right_operand)
        .ok_or_else(|| CoreError::Runtime("integer overflow in subtraction".into()))?;
    Ok(Value::new(result_type_id, value))
}

#[cfg(test)]
#[path = "subtraction_integer128.tests.rs"]
mod tests;
