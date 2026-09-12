use language_core::{BinaryOperator, CoreError, ExecutionContext, Value};

use crate::integer::integer64::value::{
    get, is_overflow_error, mixed_operands, promote_overflow_to_int128,
};

/// Subtracts two integers and reports overflow as a language error.
pub(crate) fn subtraction_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)?
        .checked_sub(get(rhs)?)
        .ok_or_else(|| CoreError::Runtime("integer overflow in subtraction".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Subtracts two integers, promoting overflowed results to `int128`.
pub(crate) fn subtraction_integer_with_context(
    context: &ExecutionContext<'_>,
    lhs: &Value,
    rhs: &Value,
) -> Result<Value, CoreError> {
    match subtraction_integer(lhs, rhs) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            promote_overflow_to_int128(context, get(lhs)?, get(rhs)?, BinaryOperator::Subtraction)
        }
        Err(error) => Err(error),
    }
}

/// Subtracts mixed-width integers after losslessly promoting both operands to `int64`.
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

/// Subtracts mixed-width integers, promoting overflowed results to `int128`.
pub(crate) fn subtraction_mixed_integer_with_context(
    context: &ExecutionContext<'_>,
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    match subtraction_mixed_integer(left_operand, right_operand) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            let (left_integer, right_integer, _) = mixed_operands(left_operand, right_operand)?;
            promote_overflow_to_int128(
                context,
                left_integer,
                right_integer,
                BinaryOperator::Subtraction,
            )
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "subtraction_integer64.tests.rs"]
mod tests;
