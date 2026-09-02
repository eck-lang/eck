use language_core::{CoreError, ExecutionContext, Value};

use crate::string::value::get;

/// Trims the string receiver and collapses consecutive Unicode whitespace into a single space.
pub(crate) fn normalize_space(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    let receiver = arguments
        .first()
        .ok_or_else(|| CoreError::Runtime("normalize_space expects a string receiver".into()))?;
    if arguments.len() != 1 {
        return Err(CoreError::Runtime(
            "normalize_space expects exactly one argument".into(),
        ));
    }
    let text = get(receiver)?;
    let transformed = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let value = context.registry().parse_string(&transformed, None)?;
    Ok(Some(value))
}

#[cfg(test)]
#[path = "normalize_space.tests.rs"]
mod tests;
