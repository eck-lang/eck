use language_core::{CoreError, ExecutionContext, Value};

use crate::string::value::get;

/// Trims Unicode whitespace from both ends of the string receiver.
pub(crate) fn trim(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    let receiver = arguments
        .first()
        .ok_or_else(|| CoreError::Runtime("trim expects a string receiver".into()))?;
    if arguments.len() != 1 {
        return Err(CoreError::Runtime(
            "trim expects exactly one argument".into(),
        ));
    }
    let text = get(receiver)?;
    let transformed = text.trim().to_owned();
    let value = context.registry().parse_string(&transformed, None)?;
    Ok(Some(value))
}

#[cfg(test)]
#[path = "trim.tests.rs"]
mod tests;
