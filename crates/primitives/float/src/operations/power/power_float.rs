use language_core::{CoreError, Value};

use crate::value::get;

/// Raises a floating-point base to a floating-point exponent.
pub(crate) fn power_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)?.powf(get(rhs)?)))
}

#[cfg(test)]
#[path = "power_float.tests.rs"]
mod tests;
