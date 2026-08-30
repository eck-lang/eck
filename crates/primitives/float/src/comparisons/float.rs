use std::cmp::Ordering;

use language_core::{ComparisonOperator, CoreError, Registry, Value};

use super::{declare_pair, evaluate};

/// Registers every comparison operator for two float operands.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    declare_pair(
        registry,
        "float",
        "float",
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

/// Produces the IEEE-754 partial ordering of two validated float payloads.
fn compare(left_operand: &Value, right_operand: &Value) -> Result<Option<Ordering>, CoreError> {
    let left_float = left_operand.downcast_ref::<f32>().ok_or_else(|| {
        CoreError::InvalidValueRepresentation("float comparison left operand".into())
    })?;
    let right_float = right_operand.downcast_ref::<f32>().ok_or_else(|| {
        CoreError::InvalidValueRepresentation("float comparison right operand".into())
    })?;

    Ok(left_float.partial_cmp(right_float))
}

/// Returns whether two float operands are equal.
fn equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Equal))
}

/// Returns whether two float operands are different.
fn not_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::NotEqual))
}

/// Returns whether the left float operand is less than the right operand.
fn less(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Less))
}

/// Returns whether the left float operand is less than or equal to the right operand.
fn less_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::LessOrEqual))
}

/// Returns whether the left float operand is greater than the right operand.
fn greater(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Greater))
}

/// Returns whether the left float operand is greater than or equal to the right operand.
fn greater_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::GreaterOrEqual))
}

#[cfg(test)]
#[path = "float.tests.rs"]
mod tests;
