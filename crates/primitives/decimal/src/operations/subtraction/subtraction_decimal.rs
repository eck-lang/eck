use language_core::{CoreError, Value};

use crate::{operations::checked_subtraction, value::get as get_decimal};

/// Subtracts the right decimal from the left decimal.
pub(crate) fn subtraction_decimal(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    Ok(Value::new(
        left_operand.type_id(),
        checked_subtraction(get_decimal(left_operand)?, get_decimal(right_operand)?)?,
    ))
}

#[cfg(test)]
#[path = "subtraction_decimal.tests.rs"]
mod tests;
