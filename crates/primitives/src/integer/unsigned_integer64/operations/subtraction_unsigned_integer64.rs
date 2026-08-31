use language_core::{CoreError, Value};

use crate::integer::unsigned_integer64::value::get;

/// Subtracts two unsigned integers and reports underflow as a language error.
pub(crate) fn subtraction_unsigned_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)?
        .checked_sub(get(rhs)?)
        .ok_or_else(|| CoreError::Runtime("unsigned integer overflow in subtraction".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}
#[cfg(test)]
#[path = "subtraction_unsigned_integer64.tests.rs"]
mod tests;
