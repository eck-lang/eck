use language_core::{CoreError, ExecutionContext, Value};

use crate::string::value::get;

/// Converts the string receiver to its uppercase form.
pub(crate) fn uppercase(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    let receiver = arguments
        .first()
        .ok_or_else(|| CoreError::Runtime("uppercase expects a string receiver".into()))?;
    if arguments.len() != 1 {
        return Err(CoreError::Runtime(
            "uppercase expects exactly one argument".into(),
        ));
    }
    let text = get(receiver)?;
    let transformed = text.to_uppercase();
    let value = context.registry().parse_string(&transformed, None)?;
    Ok(Some(value))
}

#[cfg(test)]
#[path = "uppercase.tests.rs"]
mod tests;
