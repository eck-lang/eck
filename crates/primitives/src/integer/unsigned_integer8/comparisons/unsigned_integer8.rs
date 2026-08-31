use std::cmp::Ordering;

use language_core::{ComparisonOperator, CoreError, Registry, Value};

use super::{declare_pair, evaluate};
use crate::integer::unsigned_integer8::value::get;

/// Registers every comparison operator for two unsigned integer operands.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    declare_pair(
        registry,
        "uint8",
        "uint8",
        [
            equal,
            not_equal,
            less,
            less_or_equal,
            greater,
            greater_or_equal,
        ],
    )
}

/// Produces the total ordering of two validated unsigned integer payloads.
fn compare(left_operand: &Value, right_operand: &Value) -> Result<Ordering, CoreError> {
    Ok(get(left_operand)?.cmp(&get(right_operand)?))
}

/// Returns whether two unsigned integer operands are equal.
fn equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Equal))
}

/// Returns whether two unsigned integer operands are different.
fn not_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::NotEqual))
}

/// Returns whether the left unsigned integer operand is less than the right operand.
fn less(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Less))
}

/// Returns whether the left unsigned integer operand is less than or equal to the right operand.
fn less_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::LessOrEqual))
}

/// Returns whether the left unsigned integer operand is greater than the right operand.
fn greater(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Greater))
}

/// Returns whether the left unsigned integer operand is greater than or equal to the right operand.
fn greater_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::GreaterOrEqual))
}

#[cfg(test)]
#[path = "unsigned_integer8.tests.rs"]
mod tests;
