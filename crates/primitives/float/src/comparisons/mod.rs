mod float;
mod float_integer;

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

/// Registers float comparison relations and their mixed-type overloads.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    float::register(registry)?;
    float_integer::register(registry)?;

    Ok(())
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

/// Evaluates one comparison operator from an optional partial ordering.
///
/// `None` represents an unordered IEEE-754 comparison: only inequality is
/// true, while equality and every ordering operator are false.
fn evaluate(ordering: Option<Ordering>, operator: ComparisonOperator) -> bool {
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
