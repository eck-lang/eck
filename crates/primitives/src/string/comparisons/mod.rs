mod string;
mod string_decimal;
mod string_double;
mod string_float;
mod string_integer;

#[cfg(test)]
mod test_support;

use std::{any::Any, cmp::Ordering};

use language_core::{ComparisonExecutor, ComparisonOperator, CoreError, Registry, Value};

/// Registers every equality and ordering comparison between strings.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    string::register(registry)?;
    string_integer::register(registry)?;
    string_float::register(registry)?;
    string_double::register(registry)?;
    string_decimal::register(registry)
}

/// Declares strict equality and inequality for a string and one numeric type.
fn declare_distinct_pair<T: Any>(
    registry: &mut Registry,
    numeric_type_name: &'static str,
) -> Result<(), CoreError> {
    for (left_operand_type, right_operand_type) in
        [("string", numeric_type_name), (numeric_type_name, "string")]
    {
        registry.declare_comparison(
            ComparisonOperator::Equal,
            left_operand_type,
            right_operand_type,
            distinct_equal::<T> as ComparisonExecutor,
        )?;
        registry.declare_comparison(
            ComparisonOperator::NotEqual,
            left_operand_type,
            right_operand_type,
            distinct_not_equal::<T> as ComparisonExecutor,
        )?;
    }
    Ok(())
}

/// Returns false after validating one string and one numeric payload.
fn distinct_equal<T: Any>(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    validate_distinct_payloads::<T>(left_operand, right_operand)?;
    Ok(false)
}

/// Returns true after validating one string and one numeric payload.
fn distinct_not_equal<T: Any>(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<bool, CoreError> {
    validate_distinct_payloads::<T>(left_operand, right_operand)?;
    Ok(true)
}

/// Validates that the operands contain exactly one string and one numeric payload.
fn validate_distinct_payloads<T: Any>(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<(), CoreError> {
    let string_then_numeric = left_operand.downcast_ref::<String>().is_some()
        && right_operand.downcast_ref::<T>().is_some();
    let numeric_then_string = left_operand.downcast_ref::<T>().is_some()
        && right_operand.downcast_ref::<String>().is_some();
    if string_then_numeric || numeric_then_string {
        return Ok(());
    }
    Err(CoreError::InvalidValueRepresentation(
        "string and numeric comparison".into(),
    ))
}

/// Evaluates one comparison operator from a total lexicographic ordering.
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
