use language_core::{CoreError, Value};

use crate::float::value::get;

/// Multiplies two floating-point values.
pub(crate) fn multiplication_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)? * get(rhs)?))
}

#[cfg(test)]
#[path = "multiplication_float.tests.rs"]
mod tests;
