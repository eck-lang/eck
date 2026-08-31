use crate::IntegerExtension;
use language_core::{
    BinaryOperator, ComparisonOperator, CoreError, Extension, Registry, ValueType,
};

use super::FloatExtension;

/// Verifies every float comparison relation in one populated registry.
fn assert_comparisons_are_registered(registry: &Registry) {
    let float = registry.type_by_name("float").unwrap();

    for other_type_name in ["float", "int"] {
        let other_type = registry.type_by_name(other_type_name).unwrap();
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
                    .resolve_comparison(operator, float, other_type)
                    .is_ok()
            );
            assert!(
                registry
                    .resolve_comparison(operator, other_type, float)
                    .is_ok()
            );
        }
    }
}

/// Verifies that float literals use their native precision and formatting.
#[test]
fn float_literals_use_single_precision_and_native_formatting() {
    let mut registry = Registry::new();
    FloatExtension.register(&mut registry).unwrap();
    let float = registry.type_by_name("float").unwrap();

    let value = registry.parse_numeric("16777217", Some(float)).unwrap();

    assert_eq!(*value.downcast_ref::<f32>().unwrap(), 16_777_216.0);
    assert_eq!(registry.format_value(&value).unwrap(), "16777216");
}

/// Verifies that malformed source text is rejected as a float literal.
#[test]
fn float_literals_reject_invalid_source_text() {
    let mut registry = Registry::new();
    FloatExtension.register(&mut registry).unwrap();
    let float = registry.type_by_name("float").unwrap();

    let result = registry.parse_numeric("not-a-number", Some(float));

    assert!(matches!(result, Err(CoreError::InvalidLiteral { .. })));
}

/// Verifies all arithmetic operators that convert integer operands to float.
#[test]
fn float_registers_all_integer_promotion_operators() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    FloatExtension.register(&mut registry).unwrap();
    let integer = registry.type_by_name("int").unwrap();
    let float = registry.type_by_name("float").unwrap();

    for operator in [
        BinaryOperator::Addition,
        BinaryOperator::Subtraction,
        BinaryOperator::Multiplication,
        BinaryOperator::Division,
        BinaryOperator::Remainder,
        BinaryOperator::Power,
    ] {
        assert_eq!(
            registry
                .resolve_binary_operation(
                    operator,
                    ValueType::plain(integer),
                    ValueType::plain(float),
                )
                .unwrap()
                .output,
            ValueType::plain(float)
        );
        assert_eq!(
            registry
                .resolve_binary_operation(
                    operator,
                    ValueType::plain(float),
                    ValueType::plain(integer),
                )
                .unwrap()
                .output,
            ValueType::plain(float)
        );
    }
}

/// Verifies comparisons when integer is registered before float.
#[test]
fn float_registers_comparisons_when_integer_is_registered_first() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    FloatExtension.register(&mut registry).unwrap();

    assert_comparisons_are_registered(&registry);
}

/// Verifies comparisons when integer is registered after float.
#[test]
fn float_registers_comparisons_when_integer_is_registered_later() {
    let mut registry = Registry::new();
    FloatExtension.register(&mut registry).unwrap();
    IntegerExtension.register(&mut registry).unwrap();

    assert_comparisons_are_registered(&registry);
}
