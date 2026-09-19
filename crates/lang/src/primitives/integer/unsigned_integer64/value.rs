use crate::semantic::{CoreError, Value};

/// Extracts the unsigned 64-bit integer payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<u64, CoreError> {
    value
        .downcast_ref::<u64>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("uint64".into()))
}

/// Converts this unsigned 64-bit value into a zero-based array index.
///
/// A magnitude that does not fit the machine word size cannot address a real
/// array and is rejected instead of truncating to a wrapped index.
pub(crate) fn to_index(value: &Value) -> Result<usize, CoreError> {
    let integer = get(value)?;
    usize::try_from(integer).map_err(|_| {
        CoreError::Runtime(format!(
            "array index {integer} does not fit in a machine word"
        ))
    })
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
