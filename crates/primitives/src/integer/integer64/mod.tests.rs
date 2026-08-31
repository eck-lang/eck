use language_core::{ComparisonOperator, Extension, Registry};

use super::IntegerExtension;

/// Verifies that integer is used for unqualified integer literals.
#[test]
fn integer_is_the_default_integer_type() {
    let mut registry = Registry::new();
    IntegerExtension::new().register(&mut registry).unwrap();

    let value = registry.parse_numeric("42", None).unwrap();

    assert_eq!(value.type_id(), registry.type_by_name("int64").unwrap());
    assert_eq!(
        registry.type_by_name("int64").unwrap(),
        registry.type_by_name("int").unwrap()
    );
}

/// Verifies that integer registers every same-type comparison operator.
#[test]
fn integer_registers_all_comparisons() {
    let mut registry = Registry::new();
    IntegerExtension::new().register(&mut registry).unwrap();
    let integer = registry.type_by_name("int64").unwrap();

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
