use language_core::{CoreError, Value};

use crate::operations::float_double_operands;

/// Calculates a float/double remainder after promoting the float to double precision.
pub(crate) fn remainder_double_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, double_id) = float_double_operands(lhs, rhs)?;
    if rhs == 0.0 {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(double_id, lhs % rhs))
}

#[cfg(test)]
#[path = "remainder_double_float.tests.rs"]
mod tests;
