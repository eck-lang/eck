use language_core::{CoreError, Value};

use crate::integer::integer8::value::get;

/// Adds two integers and reports overflow as a language error.
pub(crate) fn addition_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)?
        .checked_add(get(rhs)?)
        .ok_or_else(|| CoreError::Runtime("integer overflow in addition".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

#[cfg(test)]
#[path = "addition_integer8.tests.rs"]
mod tests;
