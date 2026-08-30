use language_core::{CoreError, Value};

/// Extracts the string payload owned by a runtime value.
pub(crate) fn get(value: &Value) -> Result<&str, CoreError> {
    value
        .downcast_ref::<String>()
        .map(String::as_str)
        .ok_or_else(|| CoreError::InvalidValueRepresentation("string".into()))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
