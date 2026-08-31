use language_core::{CoreError, Value};

use crate::integer::unsigned_integer64::value::get;

/// Raises an unsigned integer to an unsigned integer exponent using checked arithmetic.
pub(crate) fn power_unsigned_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let exponent = u32::try_from(get(rhs)?).map_err(|_| {
        CoreError::Runtime("unsigned integer power exponent must fit in u32".into())
    })?;
    let value = get(lhs)?
        .checked_pow(exponent)
        .ok_or_else(|| CoreError::Runtime("unsigned integer overflow in power".into()))?;
    Ok(Value::new(lhs.type_id(), value))
}
#[cfg(test)]
#[path = "power_unsigned_integer64.tests.rs"]
mod tests;
