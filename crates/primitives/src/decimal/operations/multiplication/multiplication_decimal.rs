use language_core::{CoreError, Value};

use crate::decimal::{operations::checked_multiplication, value::get as get_decimal};

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

#[cfg(test)]
#[path = "multiplication_decimal.tests.rs"]
mod tests;
