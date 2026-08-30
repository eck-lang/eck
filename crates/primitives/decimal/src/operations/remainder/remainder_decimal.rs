use language_core::{CoreError, Value};

use crate::{operations::checked_remainder, value::get as get_decimal};

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

#[cfg(test)]
#[path = "remainder_decimal.tests.rs"]
mod tests;
