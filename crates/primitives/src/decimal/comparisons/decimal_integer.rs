use std::cmp::Ordering;

use language_core::{ComparisonOperator, CoreError, Registry, Value};
use rust_decimal::Decimal;

use super::{declare_pair, evaluate};

/// Registers decimal–integer comparisons in both operand orders.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    let executors = [
        equal,
        not_equal,
        less,
        less_or_equal,
        greater,
        greater_or_equal,
    ];
    declare_pair(registry, "decimal", "int", executors)?;
    declare_pair(registry, "int", "decimal", executors)
}

/// Compares a decimal and an integer after converting the integer exactly to decimal.
///
/// The returned ordering follows the original operand order.
fn compare(left_operand: &Value, right_operand: &Value) -> Result<Option<Ordering>, CoreError> {
    if let (Some(decimal), Some(integer)) = (
        left_operand.downcast_ref::<Decimal>(),
        right_operand.downcast_ref::<i64>(),
    ) {
        return Ok(Some(decimal.cmp(&Decimal::from(*integer))));
    }
    if let (Some(integer), Some(decimal)) = (
        left_operand.downcast_ref::<i64>(),
        right_operand.downcast_ref::<Decimal>(),
    ) {
        return Ok(Some(Decimal::from(*integer).cmp(decimal)));
    }

    Err(CoreError::InvalidValueRepresentation(
        "decimal and int comparison operands".into(),
    ))
}

/// Returns whether the decimal and integer operands are equal.
fn equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Equal))
}

/// Returns whether the decimal and integer operands are different.
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
#[path = "decimal_integer.tests.rs"]
mod tests;
