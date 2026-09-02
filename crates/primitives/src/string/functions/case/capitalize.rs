use language_core::{CoreError, ExecutionContext, Value};

use crate::string::value::get;

/// Converts the string receiver to its capitalized form.
///
/// The first character is converted to uppercase, and the remaining characters
/// are converted to lowercase. The transformation is Unicode-aware and operates
/// on scalar values.
pub(crate) fn capitalize(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    let receiver = arguments
        .first()
        .ok_or_else(|| CoreError::Runtime("capitalize expects a string receiver".into()))?;
    if arguments.len() != 1 {
        return Err(CoreError::Runtime(
            "capitalize expects exactly one argument".into(),
        ));
    }
    let text = get(receiver)?;
    let transformed = if text.is_empty() {
        String::new()
    } else {
        let mut chars = text.chars();
        let first = chars.next().expect("non-empty string has first char");
        let first_upper: String = first.to_uppercase().collect();
        let rest = chars.as_str().to_lowercase();
        format!("{first_upper}{rest}")
    };
    let value = context.registry().parse_string(&transformed, None)?;
    Ok(Some(value))
}

#[cfg(test)]
#[path = "capitalize.tests.rs"]
mod tests;
