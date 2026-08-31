use language_core::{ComparisonOperator, CoreError, Registry, Value};

use crate::boolean::value::get;

/// Registers equality and inequality for two boolean operands.
///
/// Boolean values deliberately do not define ordering operators.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    registry.declare_comparison(ComparisonOperator::Equal, "bool", "bool", equal)?;
    registry.declare_comparison(ComparisonOperator::NotEqual, "bool", "bool", not_equal)
}

/// Returns whether two boolean operands are equal.
fn equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    Ok(get(left_operand)? == get(right_operand)?)
}

/// Returns whether two boolean operands are different.
fn not_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    Ok(get(left_operand)? != get(right_operand)?)
}

#[cfg(test)]
#[path = "boolean.tests.rs"]
mod tests;
