use crate::semantic::{CoreError, Value};

/// Extracts the unsigned 8-bit integer payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<u8, CoreError> {
    value
        .downcast_ref::<u8>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("uint8".into()))
}

/// Converts this unsigned 8-bit value into a zero-based array index.
pub(crate) fn to_index(value: &Value) -> Result<usize, CoreError> {
    Ok(usize::from(get(value)?))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
