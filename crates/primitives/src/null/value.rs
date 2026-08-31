use language_core::{CoreError, Value};

/// Represents the singleton null payload stored inside an opaque runtime value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Null;

/// Extracts the null payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<Null, CoreError> {
    value
        .downcast_ref::<Null>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("null".into()))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
