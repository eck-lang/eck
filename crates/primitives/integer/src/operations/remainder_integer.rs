use language_core::{CoreError, Value};

use crate::value::get;

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
#[cfg(test)]
#[path = "remainder_integer.tests.rs"]
mod tests;
