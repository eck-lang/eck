use language_core::{ComparisonOperator, Extension, Registry, ValueType};
use primitives::BoolExtension;
use primitives::DecimalExtension;
use primitives::IntegerExtension;

use crate::percentage::PercentageMeasureExtension;

#[test]
fn percentage_comparisons_are_qualified_only() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    BoolExtension.register(&mut registry).unwrap();
    PercentageMeasureExtension.register(&mut registry).unwrap();
    let integer = registry.type_by_name("int").unwrap();
    let percentage = registry.subtype_by_suffix("%").unwrap();
    assert!(
        registry
            .resolve_comparison_operation(
                ComparisonOperator::Greater,
                ValueType::qualified(integer, percentage),
                ValueType::qualified(integer, percentage)
            )
            .is_ok()
    );
    assert!(
        registry
            .resolve_comparison_operation(
                ComparisonOperator::Equal,
                ValueType::qualified(integer, percentage),
                ValueType::plain(integer)
            )
            .is_err()
    );
}
