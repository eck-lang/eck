use language_core::{CoreError, Value};

use crate::integer::bigint::value::get;

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

#[cfg(test)]
#[path = "power_bigint.tests.rs"]
mod tests;
