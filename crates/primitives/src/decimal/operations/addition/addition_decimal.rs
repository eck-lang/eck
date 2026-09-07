use language_core::{CoreError, Value};

use crate::decimal::{
    operations::checked_addition,
    value::{get as get_decimal, get_mut as get_decimal_mut},
};

/// Adds two decimal values and returns a decimal result.
pub(crate) fn addition_decimal(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    Ok(Value::new(
        left_operand.type_id(),
        checked_addition(get_decimal(left_operand)?, get_decimal(right_operand)?)?,
    ))
}

/// Adds a decimal right operand into a uniquely owned decimal left operand.
pub(crate) fn addition_decimal_in_place(
    left_operand: &mut Value,
    right_operand: &Value,
) -> Result<(), CoreError> {
    let right_operand = get_decimal(right_operand)?;
    let left_operand = get_decimal_mut(left_operand)?;
    *left_operand = checked_addition(*left_operand, right_operand)?;
    Ok(())
}

#[cfg(test)]
#[path = "addition_decimal.tests.rs"]
mod tests;
