use language_core::{CoreError, Value};

use crate::integer::integer64::value::get;

/// Multiplies two integers and reports overflow as a language error.
pub(crate) fn multiplication_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)?
        .checked_mul(get(rhs)?)
        .ok_or_else(|| CoreError::Runtime("integer overflow in multiplication".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

#[cfg(test)]
#[path = "multiplication_integer64.tests.rs"]
mod tests;
