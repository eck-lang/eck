use language_core::{CoreError, Value};

use crate::value::get;

/// Subtracts the right double-precision value from the left value.
pub(crate) fn subtraction_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)? - get(rhs)?))
}

#[cfg(test)]
#[path = "subtraction_double.tests.rs"]
mod tests;
