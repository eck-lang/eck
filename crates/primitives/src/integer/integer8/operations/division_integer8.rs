use language_core::{BinaryOperator, CoreError, ExecutionContext, Value};

use crate::integer::integer8::value::{get, is_overflow_error, promote_overflow_to_int16};

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

/// Divides two integers, promoting overflowed results to `int16`.
///
/// The runtime prefers this registry-aware implementation over
/// `division_integer`; zero divisors still report without promotion while the
/// single overflow case (`MIN / -1`) recomputes with wider precision.
pub(crate) fn division_integer_with_context(
    context: &ExecutionContext<'_>,
    lhs: &Value,
    rhs: &Value,
) -> Result<Value, CoreError> {
    match division_integer(lhs, rhs) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            promote_overflow_to_int16(context, get(lhs)?, get(rhs)?, BinaryOperator::Division)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "division_integer8.tests.rs"]
mod tests;
