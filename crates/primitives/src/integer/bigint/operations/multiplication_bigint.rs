use language_core::{CoreError, Value};

use crate::integer::bigint::value::get;

/// Multiplies two arbitrary-precision integers.
///
/// Arbitrary precision means multiplication cannot overflow; only available
/// memory limits the result.
pub(crate) fn multiplication_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)? * get(rhs)?;
    Ok(Value::new(lhs.type_id(), value))
}

#[cfg(test)]
#[path = "multiplication_bigint.tests.rs"]
mod tests;
