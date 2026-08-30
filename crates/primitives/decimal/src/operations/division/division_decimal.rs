use language_core::{CoreError, Value};

use crate::{operations::checked_division, value::get as get_decimal};

/// Divides the left decimal by the right decimal.
pub(crate) fn division_decimal(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let right_operand = get_decimal(right_operand)?;
    if right_operand.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(
        left_operand.type_id(),
        checked_division(get_decimal(left_operand)?, right_operand)?,
    ))
}

#[cfg(test)]
#[path = "division_decimal.tests.rs"]
mod tests;
