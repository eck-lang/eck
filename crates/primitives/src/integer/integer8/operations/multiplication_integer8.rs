use language_core::{BinaryOperator, CoreError, ExecutionContext, Value};

use crate::integer::integer8::value::{get, is_overflow_error, promote_overflow_to_int16};

/// Multiplies two integers and reports overflow as a language error.
pub(crate) fn multiplication_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)?
        .checked_mul(get(rhs)?)
        .ok_or_else(|| CoreError::Runtime("integer overflow in multiplication".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Multiplies two integers, promoting overflowed results to `int16`.
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
        Err(error) if is_overflow_error(&error) => promote_overflow_to_int16(
            context,
            get(lhs)?,
            get(rhs)?,
            BinaryOperator::Multiplication,
        ),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "multiplication_integer8.tests.rs"]
mod tests;
