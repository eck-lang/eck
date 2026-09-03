use language_core::{CoreError, Value};

use crate::integer::integer16::value::get;
use crate::integer::integer16::value::mixed_operands;

/// Raises an integer to a non-negative integer exponent using checked arithmetic.
pub(crate) fn power_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let exponent = get(rhs)?;
    let exponent = u32::try_from(exponent).map_err(|_| {
        CoreError::Runtime("integer power exponent must be non-negative and fit in u32".into())
    })?;
    let value = get(lhs)?
        .checked_pow(exponent)
        .ok_or_else(|| CoreError::Runtime("integer overflow in power".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Raises mixed-width integers after losslessly promoting both operands to `int16`.
pub(crate) fn power_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    let exponent = u32::try_from(right_operand).map_err(|_| {
        CoreError::Runtime("integer power exponent must be non-negative and fit in u32".into())
    })?;
    let value = left_operand
        .checked_pow(exponent)
        .ok_or_else(|| CoreError::Runtime("integer overflow in power".into()))?;
    Ok(Value::new(result_type_id, value))
}

#[cfg(test)]
#[path = "power_integer16.tests.rs"]
mod tests;
