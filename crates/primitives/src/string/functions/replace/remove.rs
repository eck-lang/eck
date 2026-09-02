use language_core::{CoreError, ExecutionContext, Value};

use crate::string::value::get;

/// Removes all occurrences of `target` from the string receiver.
pub(crate) fn remove(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    if arguments.len() != 2 {
        return Err(CoreError::Runtime(
            "remove expects a string receiver and a target".into(),
        ));
    }
    let text = get(&arguments[0])?;
    let target = get(&arguments[1])?;
    if target.is_empty() {
        return Err(CoreError::Runtime(
            "remove target must not be empty".into(),
        ));
    }
    let transformed = text.replace(target, "");
    let value = context.registry().parse_string(&transformed, None)?;
    Ok(Some(value))
}

#[cfg(test)]
#[path = "remove.tests.rs"]
mod tests;
