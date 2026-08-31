use language_core::{CoreError, Value};

use crate::integer::value::get;

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
#[cfg(test)]
#[path = "division_integer.tests.rs"]
mod tests;
