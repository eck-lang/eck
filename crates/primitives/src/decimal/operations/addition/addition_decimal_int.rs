use language_core::{CoreError, Value};

use crate::decimal::operations::{checked_addition, decimal_int_operands};

/// Adds one decimal and one integer, regardless of operand order.
pub(crate) fn addition_decimal_int(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, decimal_id) =
        decimal_int_operands(left_operand, right_operand)?;
    Ok(Value::new(
        decimal_id,
        checked_addition(left_operand, right_operand)?,
    ))
}

#[cfg(test)]
#[path = "addition_decimal_int.tests.rs"]
mod tests;
