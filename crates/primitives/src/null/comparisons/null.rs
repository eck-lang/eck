use language_core::{ComparisonOperator, CoreError, Registry, Value};

use crate::null::value::get;

/// Registers equality and inequality for two null operands.
///
/// Null deliberately does not define ordering operators.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    registry.declare_comparison(ComparisonOperator::Equal, "null", "null", equal)?;
    registry.declare_comparison(ComparisonOperator::NotEqual, "null", "null", not_equal)
}

/// Returns whether two null operands are equal.
fn equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    get(left_operand)?;
    get(right_operand)?;
    Ok(true)
}

/// Returns whether two null operands are different.
fn not_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    get(left_operand)?;
    get(right_operand)?;
    Ok(false)
}

#[cfg(test)]
#[path = "null.tests.rs"]
mod tests;
