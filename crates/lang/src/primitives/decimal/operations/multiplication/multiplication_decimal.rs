use crate::semantic::{CoreError, Value};

use crate::primitives::decimal::{
    operations::checked_multiplication,
    value::{get as get_decimal, get_mut as get_decimal_mut},
};

/// Multiplies two decimal values and returns a decimal result.
pub(crate) fn multiplication_decimal(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    Ok(Value::new(
        left_operand.type_id(),
        checked_multiplication(get_decimal(left_operand)?, get_decimal(right_operand)?)?,
    ))
}

/// Multiplies a uniquely owned decimal left operand by a decimal right operand.
pub(crate) fn multiplication_decimal_in_place(
    left_operand: &mut Value,
    right_operand: &Value,
) -> Result<(), CoreError> {
    let right_operand = get_decimal(right_operand)?;
    let left_operand = get_decimal_mut(left_operand)?;
    *left_operand = checked_multiplication(*left_operand, right_operand)?;
    Ok(())
}

#[cfg(test)]
#[path = "multiplication_decimal.tests.rs"]
mod tests;
