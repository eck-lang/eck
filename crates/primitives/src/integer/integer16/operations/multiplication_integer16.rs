use language_core::{BinaryOperator, CoreError, ExecutionContext, Value};

use crate::integer::integer16::value::get;
use crate::integer::integer16::value::{
    is_overflow_error, mixed_operands, promote_overflow_to_int32,
};

/// Multiplies two integers and reports overflow as a language error.
pub(crate) fn multiplication_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)?
        .checked_mul(get(rhs)?)
        .ok_or_else(|| CoreError::Runtime("integer overflow in multiplication".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Multiplies two integers, promoting overflowed results to `int32`.
///
/// The runtime prefers this registry-aware implementation over
/// `multiplication_integer`; overflow recomputes with wider precision while
/// invalid representations still report errors.
pub(crate) fn multiplication_integer_with_context(
    context: &ExecutionContext<'_>,
    lhs: &Value,
    rhs: &Value,
) -> Result<Value, CoreError> {
    match multiplication_integer(lhs, rhs) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => promote_overflow_to_int32(
            context,
            get(lhs)?,
            get(rhs)?,
            BinaryOperator::Multiplication,
        ),
        Err(error) => Err(error),
    }
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

/// Multiplies mixed-width integers, promoting overflowed results to `int32`.
///
/// The runtime prefers this registry-aware implementation over
/// `multiplication_mixed_integer`; operand extraction errors still report
/// without promotion while `int16` overflow recomputes with wider precision.
pub(crate) fn multiplication_mixed_integer_with_context(
    context: &ExecutionContext<'_>,
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    match multiplication_mixed_integer(left_operand, right_operand) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            let (left_integer, right_integer, _) = mixed_operands(left_operand, right_operand)?;
            promote_overflow_to_int32(
                context,
                left_integer,
                right_integer,
                BinaryOperator::Multiplication,
            )
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "multiplication_integer16.tests.rs"]
mod tests;
