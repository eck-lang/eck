use language_core::{CoreError, Value};

use crate::integer::bigint::value::get;

/// Adds two arbitrary-precision integers.
///
/// Arbitrary precision means addition cannot overflow; only available memory
/// limits the result.
pub(crate) fn addition_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)? + get(rhs)?;
    Ok(Value::new(lhs.type_id(), value))
}

#[cfg(test)]
#[path = "addition_bigint.tests.rs"]
mod tests;
