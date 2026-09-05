use language_core::{CoreError, Value};
use num_bigint::Sign;

use crate::integer::bigint::value::{get, mixed_operands};

/// Calculates the arbitrary-precision integer remainder, rejecting zero divisors.
pub(crate) fn remainder_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let divisor = get(rhs)?;
    if divisor.sign() == Sign::NoSign {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(lhs.type_id(), get(lhs)? % divisor))
}

/// Calculates the mixed-width integer remainder after losslessly promoting
/// both operands to `bigint`, rejecting zero divisors.
pub(crate) fn remainder_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    if right_operand.sign() == Sign::NoSign {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(result_type_id, left_operand % right_operand))
}

#[cfg(test)]
#[path = "remainder_bigint.tests.rs"]
mod tests;
