use language_core::{CoreError, ExecutionContext, Value};

use crate::string::value::get;

/// Repeats the string receiver `count` times.
///
/// Expects `repeat(string, count)` where `count` is a non-negative integer.
/// Returns an empty string when `count` is zero.
pub(crate) fn repeat(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    if arguments.len() != 2 {
        return Err(CoreError::Runtime(
            "repeat expects a string receiver and an integer count".into(),
        ));
    }
    let text = get(&arguments[0])?;
    let count = extract_integer(&arguments[1], "repeat count")?;
    if count < 0 {
        return Err(CoreError::Runtime(
            "repeat count must be non-negative".into(),
        ));
    }
    let count = count as usize;
    if count == 0 || text.is_empty() {
        let value = context.registry().parse_string("", None)?;
        return Ok(Some(value));
    }
    let capacity = text
        .len()
        .checked_mul(count)
        .ok_or_else(|| CoreError::Runtime("string repetition result is too large".into()))?;
    let mut result = String::new();
    result
        .try_reserve_exact(capacity)
        .map_err(|_| CoreError::Runtime("string repetition result is too large".into()))?;
    for _ in 0..count {
        result.push_str(text);
    }
    let value = context.registry().parse_string(&result, None)?;
    Ok(Some(value))
}

/// Extracts an `i64` from an integer-typed runtime value.
fn extract_integer(value: &Value, label: &str) -> Result<i64, CoreError> {
    value
        .downcast_ref::<i64>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation(label.into()))
}

#[cfg(test)]
#[path = "repeat.tests.rs"]
mod tests;
