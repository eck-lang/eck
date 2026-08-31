use language_core::{CoreError, Value};

/// Extracts the signed 64-bit integer payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<i64, CoreError> {
    value
        .downcast_ref::<i64>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int64".into()))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
