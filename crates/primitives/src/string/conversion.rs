use language_core::{CoreError, ExecutionContext, Value};

/// Converts one registered value to the default string through its formatter.
pub(crate) fn format_as_string(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    let value = arguments
        .first()
        .ok_or_else(|| CoreError::Runtime("string expects one argument".into()))?;
    let formatted = context.format_value(value)?;
    context.registry().parse_string(&formatted, None).map(Some)
}

#[cfg(test)]
#[path = "conversion.tests.rs"]
mod tests;
