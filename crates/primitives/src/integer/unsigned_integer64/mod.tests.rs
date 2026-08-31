use language_core::{ComparisonOperator, Extension, Registry};

use super::UnsignedIntegerExtension;

/// Verifies that unsigned integer can be resolved by name after registration.
#[test]
fn unsigned_integer_is_registered_by_name() {
    let mut registry = Registry::new();
    UnsignedIntegerExtension::new()
        .register(&mut registry)
        .unwrap();

    let value = registry
        .parse_numeric("42", Some(registry.type_by_name("uint64").unwrap()))
        .unwrap();

    assert_eq!(value.type_id(), registry.type_by_name("uint64").unwrap());
    assert_eq!(
        registry.type_by_name("uint64").unwrap(),
        registry.type_by_name("uint").unwrap()
    );
}

/// Verifies that unsigned integer registers every same-type comparison operator.
#[test]
fn unsigned_integer_registers_all_comparisons() {
    let mut registry = Registry::new();
    UnsignedIntegerExtension::new()
        .register(&mut registry)
        .unwrap();
    let unsigned_integer = registry.type_by_name("uint64").unwrap();

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
                .resolve_comparison(operator, unsigned_integer, unsigned_integer)
                .is_ok()
        );
    }
}
