mod integer64;
mod mixed;

use std::cmp::Ordering;

use language_core::{ComparisonExecutor, ComparisonOperator, CoreError, Registry};

const OPERATORS: [ComparisonOperator; 6] = [
    ComparisonOperator::Equal,
    ComparisonOperator::NotEqual,
    ComparisonOperator::Less,
    ComparisonOperator::LessOrEqual,
    ComparisonOperator::Greater,
    ComparisonOperator::GreaterOrEqual,
];

/// Registers the integer comparison relation.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    integer64::register(registry)
}

/// Declares every mixed-width comparison whose wider operand is `int64`.
pub(crate) fn register_promotions(registry: &mut Registry) -> Result<(), CoreError> {
    mixed::register(registry)
}

/// Declares all comparison operators for one ordered pair of operand types.
fn declare_pair(
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
fn evaluate(ordering: Ordering, operator: ComparisonOperator) -> bool {
    match operator {
        ComparisonOperator::Equal => ordering == Ordering::Equal,
        ComparisonOperator::NotEqual => ordering != Ordering::Equal,
        ComparisonOperator::Less => ordering == Ordering::Less,
        ComparisonOperator::LessOrEqual => {
            matches!(ordering, Ordering::Less | Ordering::Equal)
        }
        ComparisonOperator::Greater => ordering == Ordering::Greater,
        ComparisonOperator::GreaterOrEqual => {
            matches!(ordering, Ordering::Greater | Ordering::Equal)
        }
    }
}
