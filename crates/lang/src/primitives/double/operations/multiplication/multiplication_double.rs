use crate::semantic::{CoreError, Value};

use crate::primitives::double::value::get;

/// Multiplies two double-precision floating-point values.
pub(crate) fn multiplication_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)? * get(rhs)?))
}

#[cfg(test)]
#[path = "multiplication_double.tests.rs"]
mod tests;
