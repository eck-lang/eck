//! Shared comparison registration helpers reused by primitive comparison modules.

use std::cmp::Ordering;

use language_core::{ComparisonExecutor, ComparisonOperator, CoreError, Registry};

/// The comparison operators declared for every primitive relation.
///
/// The order is the executor order expected by [`declare_pair`].
const OPERATORS: [ComparisonOperator; 6] = [
    ComparisonOperator::Equal,
    ComparisonOperator::NotEqual,
    ComparisonOperator::Less,
    ComparisonOperator::LessOrEqual,
    ComparisonOperator::Greater,
    ComparisonOperator::GreaterOrEqual,
];

/// Declares all six comparison operators for one ordered operand-type pair.
///
/// `executors` must contain one executor per position in the comparison
/// operator order: equal, not equal, less, less or equal, greater, greater or
/// equal. The declaration is resolved by type name, so it also activates when
/// only one of the two operand types is registered yet.
pub(crate) fn declare_pair(
    registry: &mut Registry,
    left_operand_type_name: &'static str,
    right_operand_type_name: &'static str,
    executors: [ComparisonExecutor; 6],
) -> Result<(), CoreError> {
    for (operator, execute) in OPERATORS.into_iter().zip(executors) {
        registry.declare_comparison(
            operator,
            left_operand_type_name,
            right_operand_type_name,
            execute,
        )?;
    }
    Ok(())
}

/// Evaluates one comparison operator from a total ordering.
pub(crate) fn evaluate_total_order(ordering: Ordering, operator: ComparisonOperator) -> bool {
    evaluate_partial_order(Some(ordering), operator)
}

/// Evaluates one comparison operator from an optional partial ordering.
///
/// `None` represents an unordered IEEE-754 comparison: only inequality is
/// true, while equality and every ordering operator are false.
pub(crate) fn evaluate_partial_order(
    ordering: Option<Ordering>,
    operator: ComparisonOperator,
) -> bool {
    match operator {
        ComparisonOperator::Equal => ordering == Some(Ordering::Equal),
        ComparisonOperator::NotEqual => ordering != Some(Ordering::Equal),
        ComparisonOperator::Less => ordering == Some(Ordering::Less),
        ComparisonOperator::LessOrEqual => {
            matches!(ordering, Some(Ordering::Less | Ordering::Equal))
        }
        ComparisonOperator::Greater => ordering == Some(Ordering::Greater),
        ComparisonOperator::GreaterOrEqual => {
            matches!(ordering, Some(Ordering::Greater | Ordering::Equal))
        }
    }
}

#[cfg(test)]
#[path = "comparison.tests.rs"]
mod tests;
