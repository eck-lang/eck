//! Comparison rules for percentage values.

use crate::semantic::{ComparisonOperator, CoreError, Registry, SubtypeComparisonRule, SubtypeId};

pub(crate) fn register(registry: &mut Registry, percentage: SubtypeId) -> Result<(), CoreError> {
    for operator in [
        ComparisonOperator::Equal,
        ComparisonOperator::NotEqual,
        ComparisonOperator::Less,
        ComparisonOperator::LessOrEqual,
        ComparisonOperator::Greater,
        ComparisonOperator::GreaterOrEqual,
    ] {
        registry.register_subtype_comparison_rule(
            operator,
            Some(percentage),
            Some(percentage),
            SubtypeComparisonRule::new(),
        )?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "comparisons.tests.rs"]
mod tests;
