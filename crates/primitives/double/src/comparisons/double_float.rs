use std::cmp::Ordering;

use language_core::{ComparisonOperator, CoreError, Registry, Value};

use super::{declare_pair, evaluate};

/// Registers double-float comparisons in both operand orders.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    let executors = [
        equal,
        not_equal,
        less,
        less_or_equal,
        greater,
        greater_or_equal,
    ];
    declare_pair(registry, "double", "float", executors)?;
    declare_pair(registry, "float", "double", executors)
}

/// Compares a double and a float after converting the float exactly to double.
///
/// Every IEEE-754 single-precision value has an exact double-precision
/// representation. The returned ordering follows the original operand order.
fn compare(left_operand: &Value, right_operand: &Value) -> Result<Option<Ordering>, CoreError> {
    if let (Some(double), Some(float)) = (
        left_operand.downcast_ref::<f64>(),
        right_operand.downcast_ref::<f32>(),
    ) {
        return Ok(double.partial_cmp(&f64::from(*float)));
    }
    if let (Some(float), Some(double)) = (
        left_operand.downcast_ref::<f32>(),
        right_operand.downcast_ref::<f64>(),
    ) {
        return Ok(f64::from(*float).partial_cmp(double));
    }

    Err(CoreError::InvalidValueRepresentation(
        "double and float comparison operands".into(),
    ))
}

/// Returns whether the double and float operands are equal.
fn equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Equal))
}

/// Returns whether the double and float operands are different.
fn not_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::NotEqual))
}

/// Returns whether the left operand is less than the right operand.
fn less(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Less))
}

/// Returns whether the left operand is less than or equal to the right operand.
fn less_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::LessOrEqual))
}

/// Returns whether the left operand is greater than the right operand.
fn greater(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Greater))
}

/// Returns whether the left operand is greater than or equal to the right operand.
fn greater_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::GreaterOrEqual))
}

#[cfg(test)]
#[path = "double_float.tests.rs"]
mod tests;
