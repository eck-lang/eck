use language_core::{CoreError, Value};
use num_bigint::Sign;

use crate::integer::bigint::value::get;

/// Divides two arbitrary-precision integers, rejecting zero divisors.
///
/// Division truncates toward zero. Arbitrary precision means non-zero
/// division cannot overflow; only available memory limits the result.
pub(crate) fn division_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let divisor = get(rhs)?;
    if divisor.sign() == Sign::NoSign {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(lhs.type_id(), get(lhs)? / divisor))
}

#[cfg(test)]
#[path = "division_bigint.tests.rs"]
mod tests;
