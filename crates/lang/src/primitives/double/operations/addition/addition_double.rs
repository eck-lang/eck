use crate::semantic::{CoreError, Value};

use crate::primitives::double::value::get;

/// Adds two double-precision floating-point values.
pub(crate) fn addition_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)? + get(rhs)?))
}

#[cfg(test)]
#[path = "addition_double.tests.rs"]
mod tests;
