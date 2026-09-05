use language_core::{BinaryOperator, CoreError, ExecutionContext, Value};

use crate::integer::integer128::value::{
    get, is_overflow_error, mixed_operands, promote_overflow_to_bigint,
};

/// Raises an integer to a non-negative integer exponent using checked arithmetic.
pub(crate) fn power_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let exponent = get(rhs)?;
    let exponent = u32::try_from(exponent).map_err(|_| {
        CoreError::Runtime("integer power exponent must be non-negative and fit in u32".into())
    })?;
    let value = get(lhs)?
        .checked_pow(exponent)
        .ok_or_else(|| CoreError::Runtime("integer overflow in power".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Raises an integer to an exponent, promoting overflowed results to `bigint`.
///
/// The runtime prefers this registry-aware implementation over
/// `power_integer`; invalid exponents still report without promotion while
/// `int128` overflow recomputes with arbitrary precision.
pub(crate) fn power_integer_with_context(
    context: &ExecutionContext<'_>,
    lhs: &Value,
    rhs: &Value,
) -> Result<Value, CoreError> {
    match power_integer(lhs, rhs) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            promote_overflow_to_bigint(context, get(lhs)?, get(rhs)?, BinaryOperator::Power)
        }
        Err(error) => Err(error),
    }
}

/// Raises mixed-width integers after losslessly promoting both operands to `int128`.
pub(crate) fn power_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    let exponent = u32::try_from(right_operand).map_err(|_| {
        CoreError::Runtime("integer power exponent must be non-negative and fit in u32".into())
    })?;
    let value = left_operand
        .checked_pow(exponent)
        .ok_or_else(|| CoreError::Runtime("integer overflow in power".into()))?;
    Ok(Value::new(result_type_id, value))
}

/// Raises mixed-width integers, promoting overflowed results to `bigint`.
///
/// The runtime prefers this registry-aware implementation over
/// `power_mixed_integer`; invalid exponents and operand extraction errors
/// still report without promotion while `int128` overflow recomputes with
/// arbitrary precision.
pub(crate) fn power_mixed_integer_with_context(
    context: &ExecutionContext<'_>,
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    match power_mixed_integer(left_operand, right_operand) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            let (left_integer, right_integer, _) = mixed_operands(left_operand, right_operand)?;
            promote_overflow_to_bigint(context, left_integer, right_integer, BinaryOperator::Power)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "power_integer128.tests.rs"]
mod tests;
