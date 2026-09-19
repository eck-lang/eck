use crate::semantic::{BinaryOperator, CoreError, ExecutionContext, Value};

use crate::primitives::integer::integer64::value::{
    get, is_overflow_error, mixed_operands, promote_overflow_to_int128,
};

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

/// Divides two integers, promoting the signed minimum-by-`-1` overflow to `int128`.
pub(crate) fn division_integer_with_context(
    context: &ExecutionContext<'_>,
    lhs: &Value,
    rhs: &Value,
) -> Result<Value, CoreError> {
    match division_integer(lhs, rhs) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            promote_overflow_to_int128(context, get(lhs)?, get(rhs)?, BinaryOperator::Division)
        }
        Err(error) => Err(error),
    }
}

/// Divides mixed-width integers after losslessly promoting both operands to `int64`.
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

/// Divides mixed-width integers, promoting any fixed-width overflow to `int128`.
pub(crate) fn division_mixed_integer_with_context(
    context: &ExecutionContext<'_>,
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    match division_mixed_integer(left_operand, right_operand) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            let (left_integer, right_integer, _) = mixed_operands(left_operand, right_operand)?;
            promote_overflow_to_int128(
                context,
                left_integer,
                right_integer,
                BinaryOperator::Division,
            )
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "division_integer64.tests.rs"]
mod tests;
