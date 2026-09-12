use language_core::{BinaryOperator, CoreError, ExecutionContext, Value};

use crate::integer::integer64::value::{
    get, is_overflow_error, mixed_operands, promote_overflow_to_int128,
};

/// Adds two integers and reports overflow as a language error.
pub(crate) fn addition_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)?
        .checked_add(get(rhs)?)
        .ok_or_else(|| CoreError::Runtime("integer overflow in addition".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Adds two integers, promoting overflowed results to `int128`.
pub(crate) fn addition_integer_with_context(
    context: &ExecutionContext<'_>,
    lhs: &Value,
    rhs: &Value,
) -> Result<Value, CoreError> {
    match addition_integer(lhs, rhs) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            promote_overflow_to_int128(context, get(lhs)?, get(rhs)?, BinaryOperator::Addition)
        }
        Err(error) => Err(error),
    }
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

/// Adds mixed-width integers, promoting overflowed results to `int128`.
pub(crate) fn addition_mixed_integer_with_context(
    context: &ExecutionContext<'_>,
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    match addition_mixed_integer(left_operand, right_operand) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            let (left_integer, right_integer, _) = mixed_operands(left_operand, right_operand)?;
            promote_overflow_to_int128(
                context,
                left_integer,
                right_integer,
                BinaryOperator::Addition,
            )
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "addition_integer64.tests.rs"]
mod tests;
