use super::*;

use language_core::{ComparisonOperator, Registry};

/// Verifies that the null extension registers its type and equality relations.
#[test]
fn registers_null_type_and_comparisons() {
    let mut registry = Registry::new();
    NullExtension.register(&mut registry).unwrap();

    let null_type = registry.type_by_name("null").unwrap();
    assert_eq!(registry.default_null().unwrap(), null_type);
    assert!(
        registry
            .resolve_comparison(
                ComparisonOperator::Equal,
                null_type,
                null_type
            )
            .is_ok()
    );
    assert!(
        registry
            .resolve_comparison(
                ComparisonOperator::NotEqual,
                null_type,
                null_type
            )
            .is_ok()
    );
}

/// Verifies that ordering comparisons are not defined for null.
#[test]
fn rejects_null_ordering_comparisons() {
    let mut registry = Registry::new();
    NullExtension.register(&mut registry).unwrap();

    let null_type = registry.type_by_name("null").unwrap();
    assert!(
        registry
            .resolve_comparison(ComparisonOperator::Less, null_type, null_type)
            .is_err()
    );
}
