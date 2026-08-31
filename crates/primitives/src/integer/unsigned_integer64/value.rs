use language_core::{CoreError, Value};

/// Extracts the unsigned 64-bit integer payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<u64, CoreError> {
    value
        .downcast_ref::<u64>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("uint64".into()))
}
#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
