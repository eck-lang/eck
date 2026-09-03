use language_core::{CoreError, Value};

use crate::integer::integer64::value::{get, mixed_operands};

/// Adds two integers and reports overflow as a language error.
pub(crate) fn addition_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)?
        .checked_add(get(rhs)?)
        .ok_or_else(|| CoreError::Runtime("integer overflow in addition".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Adds mixed-width integers after losslessly promoting both operands to `int64`.
pub(crate) fn addition_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    let value = left_operand
        .checked_add(right_operand)
        .ok_or_else(|| CoreError::Runtime("integer overflow in addition".into()))?;
    Ok(Value::new(result_type_id, value))
}

#[cfg(test)]
#[path = "addition_integer64.tests.rs"]
mod tests;
