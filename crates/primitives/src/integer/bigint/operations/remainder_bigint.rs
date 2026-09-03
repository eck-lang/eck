use language_core::{CoreError, Value};
use num_bigint::Sign;

use crate::integer::bigint::value::get;

/// Calculates the arbitrary-precision integer remainder, rejecting zero divisors.
pub(crate) fn remainder_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let divisor = get(rhs)?;
    if divisor.sign() == Sign::NoSign {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(lhs.type_id(), get(lhs)? % divisor))
}

#[cfg(test)]
#[path = "remainder_bigint.tests.rs"]
mod tests;
