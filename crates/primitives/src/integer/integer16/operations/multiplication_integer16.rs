use language_core::{CoreError, Value};

use crate::integer::integer16::value::get;
use crate::integer::integer16::value::mixed_operands;

/// Multiplies two integers and reports overflow as a language error.
pub(crate) fn multiplication_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)?
        .checked_mul(get(rhs)?)
        .ok_or_else(|| CoreError::Runtime("integer overflow in multiplication".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Multiplies mixed-width integers after losslessly promoting both operands to `int16`.
pub(crate) fn multiplication_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    let value = left_operand
        .checked_mul(right_operand)
        .ok_or_else(|| CoreError::Runtime("integer overflow in multiplication".into()))?;
    Ok(Value::new(result_type_id, value))
}

#[cfg(test)]
#[path = "multiplication_integer16.tests.rs"]
mod tests;
