use std::cmp::Ordering;

use language_core::{ComparisonOperator, CoreError, Registry, Value};
use rust_decimal::Decimal;

use super::{declare_pair, evaluate};

/// Registers every comparison operator for two decimal operands.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    declare_pair(
        registry,
        "decimal",
        "decimal",
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

/// Produces the total ordering of two validated decimal payloads.
fn compare(left_operand: &Value, right_operand: &Value) -> Result<Option<Ordering>, CoreError> {
    let left_decimal = left_operand.downcast_ref::<Decimal>().ok_or_else(|| {
        CoreError::InvalidValueRepresentation("decimal comparison left operand".into())
    })?;
    let right_decimal = right_operand.downcast_ref::<Decimal>().ok_or_else(|| {
        CoreError::InvalidValueRepresentation("decimal comparison right operand".into())
    })?;

    Ok(Some(left_decimal.cmp(right_decimal)))
}

/// Returns whether two decimal operands are equal.
fn equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Equal))
}

/// Returns whether two decimal operands are different.
fn not_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::NotEqual))
}

/// Returns whether the left decimal operand is less than the right operand.
fn less(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Less))
}

/// Returns whether the left decimal operand is less than or equal to the right operand.
fn less_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::LessOrEqual))
}

/// Returns whether the left decimal operand is greater than the right operand.
fn greater(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Greater))
}

/// Returns whether the left decimal operand is greater than or equal to the right operand.
fn greater_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::GreaterOrEqual))
}

#[cfg(test)]
#[path = "decimal.tests.rs"]
mod tests;
