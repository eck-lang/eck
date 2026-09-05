use language_core::{BinaryOperator, CoreError, ExecutionContext, Value};

use crate::integer::integer8::value::{get, is_overflow_error, promote_overflow_to_int16};

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

/// Raises an integer to an exponent, promoting overflowed results to `int16`.
///
/// The runtime prefers this registry-aware implementation over
/// `power_integer`; invalid exponents still report without promotion, results
/// fitting `int16` promote, and larger results keep the overflow error.
pub(crate) fn power_integer_with_context(
    context: &ExecutionContext<'_>,
    lhs: &Value,
    rhs: &Value,
) -> Result<Value, CoreError> {
    match power_integer(lhs, rhs) {
        Ok(value) => Ok(value),
        Err(error) if is_overflow_error(&error) => {
            promote_overflow_to_int16(context, get(lhs)?, get(rhs)?, BinaryOperator::Power)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "power_integer8.tests.rs"]
mod tests;
