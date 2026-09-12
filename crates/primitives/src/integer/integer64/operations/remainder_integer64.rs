use language_core::{BinaryOperator, CoreError, ExecutionContext, Value};

use crate::integer::integer64::value::{
    get, is_overflow_error, mixed_operands, promote_overflow_to_int128,
};

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

/// Calculates a remainder, promoting the signed minimum-by-`-1` overflow to `int128`.
pub(crate) fn remainder_integer_with_context(
    context: &ExecutionContext<'_>,
    lhs: &Value,
    rhs: &Value,
) -> Result<Value, CoreError> {
    match remainder_integer(lhs, rhs) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            promote_overflow_to_int128(context, get(lhs)?, get(rhs)?, BinaryOperator::Remainder)
        }
        Err(error) => Err(error),
    }
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

/// Calculates a mixed-width remainder, promoting any fixed-width overflow to `int128`.
pub(crate) fn remainder_mixed_integer_with_context(
    context: &ExecutionContext<'_>,
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    match remainder_mixed_integer(left_operand, right_operand) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            let (left_integer, right_integer, _) = mixed_operands(left_operand, right_operand)?;
            promote_overflow_to_int128(
                context,
                left_integer,
                right_integer,
                BinaryOperator::Remainder,
            )
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "remainder_integer64.tests.rs"]
mod tests;
