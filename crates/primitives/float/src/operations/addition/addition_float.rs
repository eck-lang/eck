use language_core::{CoreError, Value};

use crate::value::get;

/// Adds two floating-point values.
pub(crate) fn addition_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)? + get(rhs)?))
}

#[cfg(test)]
#[path = "addition_float.tests.rs"]
mod tests;
