use language_core::{CoreError, Value};
use rust_decimal::Decimal;

use crate::boolean::value::get as get_boolean;

/// Multiplies a numeric value by a boolean, treating `true` as one and `false` as zero.
///
/// The `true` path returns the original value without performing numeric multiplication.
pub(super) fn multiplication_numeric_boolean(
    numeric: &Value,
    boolean: &Value,
) -> Result<Value, CoreError> {
    if get_boolean(boolean)? {
        return Ok(numeric.clone());
    }

    if numeric.downcast_ref::<i64>().is_some() {
        return Ok(Value::new(numeric.type_id(), 0_i64));
    }
    if let Some(value) = numeric.downcast_ref::<f32>() {
        return Ok(Value::new(numeric.type_id(), value * 0.0));
    }
    if let Some(value) = numeric.downcast_ref::<f64>() {
        return Ok(Value::new(numeric.type_id(), value * 0.0));
    }
    if let Some(value) = numeric.downcast_ref::<Decimal>() {
        return Ok(Value::new(numeric.type_id(), value * Decimal::ZERO));
    }

    Err(CoreError::InvalidValueRepresentation(
        "numeric value compatible with bool multiplication".into(),
    ))
}

/// Multiplies a boolean by a numeric value, using the same semantics as `numeric * bool`.
pub(super) fn multiplication_boolean_numeric(
    boolean: &Value,
    numeric: &Value,
) -> Result<Value, CoreError> {
    multiplication_numeric_boolean(numeric, boolean)
}

#[cfg(test)]
#[path = "multiplication_boolean.tests.rs"]
mod tests;
