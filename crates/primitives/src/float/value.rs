use language_core::{CoreError, Value};

/// Extracts the single-precision floating-point payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<f32, CoreError> {
    value
        .downcast_ref::<f32>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("float".into()))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
