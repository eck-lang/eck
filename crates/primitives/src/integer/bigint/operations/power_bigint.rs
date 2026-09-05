use language_core::{CoreError, Value};

use crate::integer::bigint::value::{get, mixed_operands};

/// Raises an arbitrary-precision integer to a non-negative exponent.
///
/// The exponent must fit in `u32`; the result itself is unbounded and limited
/// only by available memory.
pub(crate) fn power_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let exponent = get(rhs)?;
    let exponent = u32::try_from(exponent).map_err(|_| {
        CoreError::Runtime("integer power exponent must be non-negative and fit in u32".into())
    })?;
    Ok(Value::new(lhs.type_id(), get(lhs)?.pow(exponent)))
}

/// Raises a mixed-width integer base to a mixed-width exponent after
/// losslessly promoting both operands to `bigint`.
///
/// The exponent must fit in `u32`; the result itself is unbounded and limited
/// only by available memory.
pub(crate) fn power_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    let exponent = u32::try_from(right_operand).map_err(|_| {
        CoreError::Runtime("integer power exponent must be non-negative and fit in u32".into())
    })?;
    Ok(Value::new(result_type_id, left_operand.pow(exponent)))
}

#[cfg(test)]
#[path = "power_bigint.tests.rs"]
mod tests;
