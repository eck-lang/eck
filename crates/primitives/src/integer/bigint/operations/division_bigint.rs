use language_core::{CoreError, Value};
use num_bigint::Sign;

use crate::integer::bigint::value::{get, mixed_operands};

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

/// Divides mixed-width integers after losslessly promoting both operands to
/// `bigint`, rejecting zero divisors.
///
/// Division truncates toward zero. Arbitrary precision means non-zero
/// division cannot overflow; only available memory limits the result.
pub(crate) fn division_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    if right_operand.sign() == Sign::NoSign {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(result_type_id, left_operand / right_operand))
}

#[cfg(test)]
#[path = "division_bigint.tests.rs"]
mod tests;
