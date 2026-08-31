use language_core::{CoreError, Value};

/// Extracts the double-precision floating-point payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<f64, CoreError> {
    value
        .downcast_ref::<f64>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("double".into()))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
