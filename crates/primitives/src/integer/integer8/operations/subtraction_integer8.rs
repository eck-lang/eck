use language_core::{BinaryOperator, CoreError, ExecutionContext, Value};

use crate::integer::integer8::value::{get, is_overflow_error, promote_overflow_to_int16};

/// Subtracts two integers and reports overflow as a language error.
pub(crate) fn subtraction_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)?
        .checked_sub(get(rhs)?)
        .ok_or_else(|| CoreError::Runtime("integer overflow in subtraction".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Subtracts two integers, promoting overflowed results to `int16`.
///
/// The runtime prefers this registry-aware implementation over
/// `subtraction_integer`; overflow recomputes with wider precision while
/// invalid representations still report errors.
pub(crate) fn subtraction_integer_with_context(
    context: &ExecutionContext<'_>,
    lhs: &Value,
    rhs: &Value,
) -> Result<Value, CoreError> {
    match subtraction_integer(lhs, rhs) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            promote_overflow_to_int16(context, get(lhs)?, get(rhs)?, BinaryOperator::Subtraction)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "subtraction_integer8.tests.rs"]
mod tests;
