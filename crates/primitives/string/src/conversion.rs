use language_core::{CoreError, Registry, Value};

/// Converts one registered value to the default string through its formatter.
pub(crate) fn format_as_string(
    registry: &Registry,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    let value = arguments
        .first()
        .ok_or_else(|| CoreError::Runtime("string expects one argument".into()))?;
    let formatted = registry.format_value(value)?;
    registry.parse_string(&formatted, None).map(Some)
}

#[cfg(test)]
#[path = "conversion.tests.rs"]
mod tests;
