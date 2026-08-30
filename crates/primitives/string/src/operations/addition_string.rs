use language_core::{CoreError, Value};

use crate::value::get;

/// Concatenates two string payloads in source order.
pub(crate) fn addition_string(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let left_text = get(left_operand)?;
    let right_text = get(right_operand)?;
    let capacity = left_text
        .len()
        .checked_add(right_text.len())
        .ok_or_else(|| CoreError::Runtime("string concatenation result is too large".into()))?;
    let mut result = String::new();
    result
        .try_reserve_exact(capacity)
        .map_err(|_| CoreError::Runtime("string concatenation result is too large".into()))?;
    result.push_str(left_text);
    result.push_str(right_text);
    Ok(Value::new(left_operand.type_id(), result))
}

#[cfg(test)]
#[path = "addition_string.tests.rs"]
mod tests;
