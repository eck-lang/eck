use language_core::{CoreError, Value};

/// Multiplies a float and an integer, preserving either source order.
pub(crate) fn multiplication_float_int(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    if let Some(float) = lhs.downcast_ref::<f32>() {
        let integer = rhs
            .downcast_ref::<i64>()
            .ok_or_else(|| CoreError::InvalidValueRepresentation("int".into()))?;
        return Ok(Value::new(lhs.type_id(), *float * *integer as f32));
    }

    if let Some(integer) = lhs.downcast_ref::<i64>() {
        let float = rhs
            .downcast_ref::<f32>()
            .ok_or_else(|| CoreError::InvalidValueRepresentation("float".into()))?;
        return Ok(Value::new(rhs.type_id(), *integer as f32 * *float));
    }

    Err(CoreError::InvalidValueRepresentation("float or int".into()))
}

#[cfg(test)]
#[path = "multiplication_float_int.tests.rs"]
mod tests;
