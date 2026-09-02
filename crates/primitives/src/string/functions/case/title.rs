use language_core::{CoreError, ExecutionContext, Value};

use crate::string::value::get;

/// Converts the string receiver to a title case.
///
/// Each word's first alphabetic character is converted to uppercase, and the
/// remaining alphabetic characters in the word are converted to lowercase.
/// Words are delimited by any non-alphabetic character, which is preserved
/// verbatim. This matches the typical `title` semantics while remaining
/// Unicode-aware.
pub(crate) fn title(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    let receiver = arguments
        .first()
        .ok_or_else(|| CoreError::Runtime("title expects a string receiver".into()))?;
    if arguments.len() != 1 {
        return Err(CoreError::Runtime(
            "title expects exactly one argument".into(),
        ));
    }
    let text = get(receiver)?;
    let mut transformed = String::with_capacity(text.len());
    let mut start_of_word = true;
    for ch in text.chars() {
        if ch.is_alphabetic() {
            if start_of_word {
                for up in ch.to_uppercase() {
                    transformed.push(up);
                }
                start_of_word = false;
            } else {
                for lo in ch.to_lowercase() {
                    transformed.push(lo);
                }
            }
        } else {
            transformed.push(ch);
            start_of_word = true;
        }
    }
    let value = context.registry().parse_string(&transformed, None)?;
    Ok(Some(value))
}

#[cfg(test)]
#[path = "title.tests.rs"]
mod tests;
