use language_core::{CoreError, Value};

use crate::integer::integer16::value::get;
use crate::integer::integer16::value::mixed_operands;

/// Divides two integers, rejecting zero divisors and overflow.
pub(crate) fn division_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let rhs = get(rhs)?;
    if rhs == 0 {
        return Err(CoreError::DivisionByZero);
    }
    let value = get(lhs)?
        .checked_div(rhs)
        .ok_or_else(|| CoreError::Runtime("integer overflow in division".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Divides mixed-width integers after losslessly promoting both operands to `int16`.
pub(crate) fn division_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    if right_operand == 0 {
        return Err(CoreError::DivisionByZero);
    }
    let value = left_operand
        .checked_div(right_operand)
        .ok_or_else(|| CoreError::Runtime("integer overflow in division".into()))?;
    Ok(Value::new(result_type_id, value))
}

#[cfg(test)]
#[path = "division_integer16.tests.rs"]
mod tests;
