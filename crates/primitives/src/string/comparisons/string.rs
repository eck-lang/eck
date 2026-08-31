use std::cmp::Ordering;

use language_core::{ComparisonOperator, CoreError, Registry, Value};

use super::evaluate;
use crate::string::value::get;

/// Registers every comparison operator for two string operands.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    for (operator, execute) in [
        (ComparisonOperator::Equal, equal as _),
        (ComparisonOperator::NotEqual, not_equal as _),
        (ComparisonOperator::Less, less as _),
        (ComparisonOperator::LessOrEqual, less_or_equal as _),
        (ComparisonOperator::Greater, greater as _),
        (ComparisonOperator::GreaterOrEqual, greater_or_equal as _),
    ] {
        registry.declare_comparison(operator, "string", "string", execute)?;
    }
    Ok(())
}

/// Produces the lexicographic ordering of two validated Unicode strings.
fn compare(left_operand: &Value, right_operand: &Value) -> Result<Ordering, CoreError> {
    Ok(get(left_operand)?.cmp(get(right_operand)?))
}

/// Returns whether two string operands are equal.
fn equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Equal))
}

/// Returns whether two string operands are different.
fn not_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::NotEqual))
}

/// Returns whether the left string is lexicographically less than the right string.
fn less(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Less))
}

/// Returns whether the left string is lexicographically at most the right string.
fn less_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::LessOrEqual))
}

/// Returns whether the left string is lexicographically greater than the right string.
fn greater(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Greater))
}

/// Returns whether the left string is lexicographically at least the right string.
fn greater_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::GreaterOrEqual))
}

#[cfg(test)]
#[path = "string.tests.rs"]
mod tests;
