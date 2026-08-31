use language_core::{CoreError, Value};

/// Extracts the signed 8-bit integer payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<i8, CoreError> {
    value
        .downcast_ref::<i8>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int8".into()))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
