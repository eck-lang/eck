use language_core::{CoreError, Value};

use crate::value::get;

/// Calculates the remainder of the left double-precision value divided by the right one.
pub(crate) fn remainder_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let rhs = get(rhs)?;
    if rhs == 0.0 {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(lhs.type_id(), get(lhs)? % rhs))
}

#[cfg(test)]
#[path = "remainder_double.tests.rs"]
mod tests;
