use language_core::{CoreError, ExecutionContext, Value};

use crate::string::value::get;

/// Converts the string receiver to its lowercase form.
pub(crate) fn lowercase(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    let receiver = arguments
        .first()
        .ok_or_else(|| CoreError::Runtime("lowercase expects a string receiver".into()))?;
    if arguments.len() != 1 {
        return Err(CoreError::Runtime(
            "lowercase expects exactly one argument".into(),
        ));
    }
    let text = get(receiver)?;
    let transformed = text.to_lowercase();
    let value = context.registry().parse_string(&transformed, None)?;
    Ok(Some(value))
}

#[cfg(test)]
#[path = "lowercase.tests.rs"]
mod tests;
