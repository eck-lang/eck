use language_core::{CoreError, Value};

/// Extracts the boolean payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<bool, CoreError> {
    value
        .downcast_ref::<bool>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("bool".into()))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
