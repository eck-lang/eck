use language_core::{CoreError, Value};

use crate::integer::bigint::value::{get, get_mut, mixed_operands};

/// Adds two arbitrary-precision integers.
///
/// Arbitrary precision means addition cannot overflow; only available memory
/// limits the result.
pub(crate) fn addition_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let value = get(lhs)? + get(rhs)?;
    Ok(Value::new(lhs.type_id(), value))
}

/// Adds a right operand into a uniquely owned `bigint` left operand.
pub(crate) fn addition_integer_in_place(
    left_operand: &mut Value,
    right_operand: &Value,
) -> Result<(), CoreError> {
    let right_operand = get(right_operand)?;
    *get_mut(left_operand)? += right_operand;
    Ok(())
}

/// Adds mixed-width integers after losslessly promoting both operands to `bigint`.
pub(crate) fn addition_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    Ok(Value::new(result_type_id, left_operand + right_operand))
}

#[cfg(test)]
#[path = "addition_bigint.tests.rs"]
mod tests;
