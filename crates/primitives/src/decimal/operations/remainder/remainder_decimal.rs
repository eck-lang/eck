use language_core::{CoreError, Value};

use crate::decimal::{
    operations::checked_remainder,
    value::{get as get_decimal, get_mut as get_decimal_mut},
};

/// Calculates the remainder of two decimal values.
pub(crate) fn remainder_decimal(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let right_operand = get_decimal(right_operand)?;
    if right_operand.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(
        left_operand.type_id(),
        checked_remainder(get_decimal(left_operand)?, right_operand)?,
    ))
}

/// Reduces a uniquely owned decimal left operand modulo a non-zero decimal right operand.
pub(crate) fn remainder_decimal_in_place(
    left_operand: &mut Value,
    right_operand: &Value,
) -> Result<(), CoreError> {
    let right_operand = get_decimal(right_operand)?;
    if right_operand.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    let left_operand = get_decimal_mut(left_operand)?;
    *left_operand = checked_remainder(*left_operand, right_operand)?;
    Ok(())
}

#[cfg(test)]
#[path = "remainder_decimal.tests.rs"]
mod tests;
