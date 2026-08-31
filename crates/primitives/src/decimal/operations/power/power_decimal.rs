use language_core::{CoreError, Value};

use crate::decimal::{
    operations::{checked_power, decimal_exponent},
    value::get as get_decimal,
};

/// Raises a decimal to an integer-valued decimal exponent.
pub(crate) fn power_decimal(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let exponent = decimal_exponent(get_decimal(right_operand)?)?;
    Ok(Value::new(
        left_operand.type_id(),
        checked_power(get_decimal(left_operand)?, exponent)?,
    ))
}

#[cfg(test)]
#[path = "power_decimal.tests.rs"]
mod tests;
