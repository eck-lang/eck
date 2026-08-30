use language_core::{CoreError, Value};

use crate::operations::float_double_operands;

/// Adds a double and a float after promoting the float to double precision.
pub(crate) fn addition_double_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, double_id) = float_double_operands(lhs, rhs)?;
    Ok(Value::new(double_id, lhs + rhs))
}

#[cfg(test)]
#[path = "addition_double_float.tests.rs"]
mod tests;
