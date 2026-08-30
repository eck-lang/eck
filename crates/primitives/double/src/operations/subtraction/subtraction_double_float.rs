use language_core::{CoreError, Value};

use crate::operations::float_double_operands;

/// Subtracts a float and a double after promoting the float to double precision.
pub(crate) fn subtraction_double_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, double_id) = float_double_operands(lhs, rhs)?;
    Ok(Value::new(double_id, lhs - rhs))
}

#[cfg(test)]
#[path = "subtraction_double_float.tests.rs"]
mod tests;
