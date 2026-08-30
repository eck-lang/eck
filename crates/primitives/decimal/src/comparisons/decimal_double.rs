use std::cmp::Ordering;

use language_core::{ComparisonOperator, CoreError, Registry, Value};
use rust_decimal::Decimal;

use super::{binary_float::compare_decimal_with_double, declare_pair, evaluate};

/// Registers decimal–double comparisons in both operand orders.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    let executors = [
        equal,
        not_equal,
        less,
        less_or_equal,
        greater,
        greater_or_equal,
    ];
    declare_pair(registry, "decimal", "double", executors)?;
    declare_pair(registry, "double", "decimal", executors)
}

/// Compares a decimal and a double without converting either value to the other type.
///
/// The returned partial ordering follows the original operand order and is
/// `None` when the double payload is NaN.
fn compare(left_operand: &Value, right_operand: &Value) -> Result<Option<Ordering>, CoreError> {
    if let (Some(decimal), Some(double)) = (
        left_operand.downcast_ref::<Decimal>(),
        right_operand.downcast_ref::<f64>(),
    ) {
        return Ok(compare_decimal_with_double(*decimal, *double));
    }
    if let (Some(double), Some(decimal)) = (
        left_operand.downcast_ref::<f64>(),
        right_operand.downcast_ref::<Decimal>(),
    ) {
        return Ok(compare_decimal_with_double(*decimal, *double).map(Ordering::reverse));
    }

    Err(CoreError::InvalidValueRepresentation(
        "decimal and double comparison operands".into(),
    ))
}

/// Returns whether the decimal and double operands are equal.
fn equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Equal))
}

/// Returns whether the decimal and double operands are different.
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
#[path = "decimal_double.tests.rs"]
mod tests;
