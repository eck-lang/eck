use crate::semantic::{CoreError, Value};

use crate::primitives::integer::bigint::value::{get, get_mut, mixed_operands};

/// Multiplies two arbitrary-precision integers.
///
/// Arbitrary precision means multiplication cannot overflow; only available
/// memory limits the result.
pub(crate) fn multiplication_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)? * get(rhs)?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Multiplies a uniquely owned `bigint` left operand by a right operand.
pub(crate) fn multiplication_integer_in_place(
    left_operand: &mut Value,
    right_operand: &Value,
) -> Result<(), CoreError> {
    let right_operand = get(right_operand)?;
    *get_mut(left_operand)? *= right_operand;
    Ok(())
}

/// Multiplies mixed-width integers after losslessly promoting both operands to `bigint`.
pub(crate) fn multiplication_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    Ok(Value::new(result_type_id, left_operand * right_operand))
}

#[cfg(test)]
#[path = "multiplication_bigint.tests.rs"]
mod tests;
