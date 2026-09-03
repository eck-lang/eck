use language_core::{ComparisonOperator, Extension, Registry};

use super::BigintExtension;

/// Verifies that bigint is registered by name and parses explicit literals.
#[test]
fn bigint_is_registered_by_name() {
    let mut registry = Registry::new();
    BigintExtension::new().register(&mut registry).unwrap();

    let value = registry
        .parse_numeric("42", Some(registry.type_by_name("bigint").unwrap()))
        .unwrap();

    assert_eq!(value.type_id(), registry.type_by_name("bigint").unwrap());
}

/// Verifies that integer registers every same-type comparison operator.
#[test]
fn integer_registers_all_comparisons() {
    let mut registry = Registry::new();
    BigintExtension::new().register(&mut registry).unwrap();
    let integer = registry.type_by_name("bigint").unwrap();

    for operator in [
        ComparisonOperator::Equal,
        ComparisonOperator::NotEqual,
        ComparisonOperator::Less,
        ComparisonOperator::LessOrEqual,
        ComparisonOperator::Greater,
        ComparisonOperator::GreaterOrEqual,
    ] {
        assert!(
            registry
                .resolve_comparison(operator, integer, integer)
                .is_ok()
        );
    }
}
