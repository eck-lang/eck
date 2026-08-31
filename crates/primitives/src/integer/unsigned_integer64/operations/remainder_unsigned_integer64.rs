use language_core::{CoreError, Value};

use crate::integer::unsigned_integer64::value::get;

/// Computes the remainder of two unsigned integers, rejecting zero divisors.
pub(crate) fn remainder_unsigned_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let divisor = get(rhs)?;
    if divisor == 0 {
        return Err(CoreError::DivisionByZero);
    }
    let value = get(lhs)?
        .checked_rem(divisor)
        .ok_or_else(|| CoreError::Runtime("unsigned integer overflow in remainder".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}
#[cfg(test)]
#[path = "remainder_unsigned_integer64.tests.rs"]
mod tests;
