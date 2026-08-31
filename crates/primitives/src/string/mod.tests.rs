use crate::IntegerExtension;
use language_core::{BinaryOperator, ComparisonOperator, Extension, Registry, ValueType};

use super::StringExtension;

/// Verifies that uncontextualized string literals use the built-in string type.
#[test]
fn string_is_the_default_string_type() {
    let mut registry = Registry::new();
    StringExtension.register(&mut registry).unwrap();

    let value = registry.parse_string("hello", None).unwrap();

    assert_eq!(value.type_id(), registry.type_by_name("string").unwrap());
    assert_eq!(value.downcast_ref::<String>().unwrap(), "hello");
}

/// Verifies registration of concatenation, integer repetition, and comparisons.
#[test]
fn registers_string_operators_and_comparisons() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    StringExtension.register(&mut registry).unwrap();
    let string_type = registry.type_by_name("string").unwrap();
    let integer_type = registry.type_by_name("int").unwrap();

    assert_eq!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::plain(string_type),
                ValueType::plain(string_type),
            )
            .unwrap()
            .output,
        ValueType::plain(string_type)
    );
    assert_eq!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Multiplication,
                ValueType::plain(string_type),
                ValueType::plain(integer_type),
            )
            .unwrap()
            .output,
        ValueType::plain(string_type)
    );
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
                .resolve_comparison(operator, string_type, string_type)
                .is_ok()
        );
    }
}

/// Verifies that the universal string function delegates to registered formatting.
#[test]
fn formats_registered_values_through_the_string_function() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    StringExtension.register(&mut registry).unwrap();
    let integer = registry.parse_numeric("42", None).unwrap();
    let function = registry
        .resolve_function("string", &[integer.type_id()])
        .unwrap();

    let result = (registry.function(function).unwrap().execute)(&registry, &[integer])
        .unwrap()
        .unwrap();

    assert_eq!(result.downcast_ref::<String>().unwrap(), "42");
}
