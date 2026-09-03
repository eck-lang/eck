use language_core::{CoreError, Value};

use crate::integer::integer64::value::{get, mixed_operands};

/// Calculates the integer remainder, rejecting zero divisors and overflow.
pub(crate) fn remainder_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let rhs = get(rhs)?;
    if rhs == 0 {
        return Err(CoreError::DivisionByZero);
    }
    let value = get(lhs)?
        .checked_rem(rhs)
        .ok_or_else(|| CoreError::Runtime("integer overflow in remainder".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Calculates a mixed-width remainder after promoting both operands to `int64`.
pub(crate) fn remainder_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    if right_operand == 0 {
        return Err(CoreError::DivisionByZero);
    }
    let value = left_operand
        .checked_rem(right_operand)
        .ok_or_else(|| CoreError::Runtime("integer overflow in remainder".into()))?;
    Ok(Value::new(result_type_id, value))
}

#[cfg(test)]
#[path = "remainder_integer64.tests.rs"]
mod tests;
