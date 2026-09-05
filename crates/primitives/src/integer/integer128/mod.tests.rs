use language_core::{ComparisonOperator, Extension, Registry};

use super::Integer128Extension;

/// Verifies that int128 is registered by name and parses explicit literals.
#[test]
fn int128_is_registered_by_name() {
    let mut registry = Registry::new();
    Integer128Extension::new().register(&mut registry).unwrap();

    let value = registry
        .parse_numeric("42", Some(registry.type_by_name("int128").unwrap()))
        .unwrap();

    assert_eq!(value.type_id(), registry.type_by_name("int128").unwrap());
}

/// Verifies that integer registers every same-type comparison operator.
#[test]
fn integer_registers_all_comparisons() {
    let mut registry = Registry::new();
    Integer128Extension::new().register(&mut registry).unwrap();
    let integer = registry.type_by_name("int128").unwrap();

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
