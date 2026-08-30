use language_core::{CoreError, Value};

use crate::{operations::checked_addition, value::get as get_decimal};

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

#[cfg(test)]
#[path = "addition_decimal.tests.rs"]
mod tests;
