use language_core::{CoreError, Value};

/// Extracts the unsigned 8-bit integer payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<u8, CoreError> {
    value
        .downcast_ref::<u8>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("uint8".into()))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
