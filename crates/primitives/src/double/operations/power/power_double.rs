use language_core::{CoreError, Value};

use crate::double::value::get;

/// Raises a double-precision floating-point base to a double-precision exponent.
pub(crate) fn power_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)?.powf(get(rhs)?)))
}

#[cfg(test)]
#[path = "power_double.tests.rs"]
mod tests;
