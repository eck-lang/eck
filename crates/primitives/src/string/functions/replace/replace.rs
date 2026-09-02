use language_core::{CoreError, ExecutionContext, Value};

use crate::string::value::get;

/// Replaces all occurrences of `target` in the string receiver with `replacement`.
pub(crate) fn replace(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    if arguments.len() != 3 {
        return Err(CoreError::Runtime(
            "replace expects a string receiver, a target, and a replacement".into(),
        ));
    }
    let text = get(&arguments[0])?;
    let target = get(&arguments[1])?;
    let replacement = get(&arguments[2])?;
    if target.is_empty() {
        return Err(CoreError::Runtime(
            "replace target must not be empty".into(),
        ));
    }
    let transformed = text.replace(target, replacement);
    let value = context.registry().parse_string(&transformed, None)?;
    Ok(Some(value))
}

#[cfg(test)]
#[path = "replace.tests.rs"]
mod tests;
