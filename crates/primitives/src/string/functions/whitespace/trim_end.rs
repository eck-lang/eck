use language_core::{CoreError, ExecutionContext, Value};

use crate::string::value::get;

/// Trims Unicode whitespace from the end of the string receiver.
pub(crate) fn trim_end(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    let receiver = arguments
        .first()
        .ok_or_else(|| CoreError::Runtime("trim_end expects a string receiver".into()))?;
    if arguments.len() != 1 {
        return Err(CoreError::Runtime(
            "trim_end expects exactly one argument".into(),
        ));
    }
    let text = get(receiver)?;
    let transformed = text.trim_end().to_owned();
    let value = context.registry().parse_string(&transformed, None)?;
    Ok(Some(value))
}

#[cfg(test)]
#[path = "trim_end.tests.rs"]
mod tests;
