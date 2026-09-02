use language_core::{CoreError, ExecutionContext, Value};

use crate::string::value::get;

/// Pads the string receiver on the end to reach `target_length` using `pad_string`.
///
/// If the receiver is already at least `target_length` characters, it is returned
/// unchanged. `pad_string` is repeated and truncated to fill the required
/// padding length. `target_length` is measured in Unicode scalar values.
pub(crate) fn pad_end(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    if arguments.len() != 3 {
        return Err(CoreError::Runtime(
            "pad_end expects a string receiver, a target length, and a pad string".into(),
        ));
    }
    let text = get(&arguments[0])?;
    let target_length = extract_integer(&arguments[1], "pad_end target length")?;
    let pad_string = get(&arguments[2])?;
    if target_length < 0 {
        return Err(CoreError::Runtime(
            "pad_end target length must be non-negative".into(),
        ));
    }
    let target_length = target_length as usize;
    let char_count = text.chars().count();
    if char_count >= target_length {
        let value = context.registry().parse_string(text, None)?;
        return Ok(Some(value));
    }
    if pad_string.is_empty() {
        return Err(CoreError::Runtime(
            "pad_end pad string must not be empty".into(),
        ));
    }
    let needed = target_length - char_count;
    let pad = build_pad(pad_string, needed);
    let transformed = format!("{text}{pad}");
    let value = context.registry().parse_string(&transformed, None)?;
    Ok(Some(value))
}

/// Extracts an `i64` from an integer-typed runtime value.
fn extract_integer(value: &Value, label: &str) -> Result<i64, CoreError> {
    value
        .downcast_ref::<i64>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation(label.into()))
}

/// Builds a padding string of exactly `needed` characters by repeating `pad_string`.
fn build_pad(pad_string: &str, needed: usize) -> String {
    let pad_chars: Vec<char> = pad_string.chars().collect();
    let mut result = String::new();
    let mut index = 0;
    while result.chars().count() < needed {
        result.push(pad_chars[index % pad_chars.len()]);
        index += 1;
    }
    result
}

#[cfg(test)]
#[path = "pad_end.tests.rs"]
mod tests;
