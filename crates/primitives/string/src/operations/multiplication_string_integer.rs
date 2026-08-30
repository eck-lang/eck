use language_core::{CoreError, Value};

use crate::value::get;

/// Repeats a string by a non-negative signed integer count.
pub(crate) fn multiplication_string_integer(
    string_operand: &Value,
    integer_operand: &Value,
) -> Result<Value, CoreError> {
    let text = get(string_operand)?;
    let count = integer_operand
        .downcast_ref::<i64>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".into()))?;
    let count = usize::try_from(count)
        .map_err(|_| CoreError::Runtime("string repetition count cannot be negative".into()))?;
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
    Ok(Value::new(string_operand.type_id(), result))
}

#[cfg(test)]
#[path = "multiplication_string_integer.tests.rs"]
mod tests;
