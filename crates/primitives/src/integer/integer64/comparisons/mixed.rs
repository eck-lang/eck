use std::cmp::Ordering;

use language_core::{ComparisonExecutor, ComparisonOperator, CoreError, Registry, Value};

use super::{declare_pair, evaluate};
use crate::integer::integer64::value::mixed_operands;

/// Declares mixed comparisons with an `int64` operand in both operand orders.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    for narrower_type_name in ["int8", "int16"] {
        let executors = executors();
        declare_pair(registry, narrower_type_name, "int64", executors)?;
        declare_pair(registry, "int64", narrower_type_name, executors)?;
    }
    Ok(())
}

/// Returns the executor set shared by every compatible ordered type pair.
fn executors() -> [ComparisonExecutor; 6] {
    [
        equal,
        not_equal,
        less,
        less_or_equal,
        greater,
        greater_or_equal,
    ]
}

/// Produces the exact ordering after promoting the narrower integer to `int64`.
fn compare(left_operand: &Value, right_operand: &Value) -> Result<Ordering, CoreError> {
    let (left_operand, right_operand, _) = mixed_operands(left_operand, right_operand)?;
    Ok(left_operand.cmp(&right_operand))
}

/// Returns whether the mixed integer operands are equal.
fn equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Equal))
}

/// Returns whether the mixed integer operands are different.
fn not_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::NotEqual))
}

/// Returns whether the left mixed integer operand is less than the right operand.
fn less(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Less))
}

/// Returns whether the left mixed integer operand is less than or equal to the right operand.
fn less_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::LessOrEqual))
}

/// Returns whether the left mixed integer operand is greater than the right operand.
fn greater(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Greater))
}

/// Returns whether the left mixed integer operand is greater than or equal to the right operand.
fn greater_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::GreaterOrEqual))
}

#[cfg(test)]
#[path = "mixed.tests.rs"]
mod tests;
