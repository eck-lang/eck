use language_core::{CoreError, Value};

use crate::integer::unsigned_integer64::value::get;

/// Divides two unsigned integers, rejecting zero divisors.
pub(crate) fn division_unsigned_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let divisor = get(rhs)?;
    if divisor == 0 {
        return Err(CoreError::DivisionByZero);
    }
    let value = get(lhs)?
        .checked_div(divisor)
        .ok_or_else(|| CoreError::Runtime("unsigned integer overflow in division".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

#[cfg(test)]
#[path = "division_unsigned_integer64.tests.rs"]
mod tests;
