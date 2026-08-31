use language_core::{CoreError, Value};

use crate::double::operations::float_double_operands;

/// Raises a float/double pair to a power after promoting the float to double precision.
pub(crate) fn power_double_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, double_id) = float_double_operands(lhs, rhs)?;
    Ok(Value::new(double_id, lhs.powf(rhs)))
}

#[cfg(test)]
#[path = "power_double_float.tests.rs"]
mod tests;
