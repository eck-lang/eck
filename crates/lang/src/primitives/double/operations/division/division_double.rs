use crate::semantic::{CoreError, Value};

use crate::primitives::double::value::get;

/// Divides the left double-precision value by the right one.
pub(crate) fn division_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let rhs = get(rhs)?;
    if rhs == 0.0 {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(lhs.type_id(), get(lhs)? / rhs))
}

#[cfg(test)]
#[path = "division_double.tests.rs"]
mod tests;
