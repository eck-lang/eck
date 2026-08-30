use super::*;
use double::DoubleExtension;
use float::FloatExtension;
use integer::IntegerExtension;
use language_core::{BinaryOperator, ComparisonOperator, Extension, Registry, ValueType};

/// Verifies that all mixed numeric comparisons resolve in both operand orders.
fn assert_mixed_comparisons_are_registered(registry: &Registry) {
    let decimal = registry.type_by_name("decimal").unwrap();

    for other_type_name in ["int", "float", "double"] {
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
                    .resolve_comparison(operator, decimal, other_type)
                    .is_ok()
            );
            assert!(
                registry
                    .resolve_comparison(operator, other_type, decimal)
                    .is_ok()
            );
        }
    }
}

/// Verifies that decimal supplies the registry's default fractional type.
#[test]
fn decimal_is_the_default_fractional_type() {
    let mut registry = Registry::new();
    DecimalExtension.register(&mut registry).unwrap();

    let value = registry.parse_numeric("0.2", None).unwrap();

    assert_eq!(value.type_id(), registry.type_by_name("decimal").unwrap());
}

/// Verifies every arithmetic operator registered between decimal and binary floats.
#[test]
fn decimal_registers_all_mixed_float_and_double_operators() {
    let mut registry = Registry::new();
    FloatExtension.register(&mut registry).unwrap();
    DoubleExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();

    let decimal = registry.type_by_name("decimal").unwrap();
    let float = registry.type_by_name("float").unwrap();
    let double = registry.type_by_name("double").unwrap();

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
                    ValueType::plain(decimal),
                    ValueType::plain(float),
                )
                .unwrap()
                .output,
            ValueType::plain(decimal)
        );
        assert_eq!(
            registry
                .resolve_binary_operation(
                    operator,
                    ValueType::plain(decimal),
                    ValueType::plain(double),
                )
                .unwrap()
                .output,
            ValueType::plain(decimal)
        );
    }
}

/// Verifies mixed comparisons when their counterpart types already exist.
#[test]
fn decimal_registers_mixed_comparisons_when_other_types_are_registered_first() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    FloatExtension.register(&mut registry).unwrap();
    DoubleExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();

    assert_mixed_comparisons_are_registered(&registry);
}

/// Verifies late binding when counterpart types register after decimal.
#[test]
fn decimal_registers_mixed_comparisons_when_other_types_are_registered_later() {
    let mut registry = Registry::new();
    DecimalExtension.register(&mut registry).unwrap();
    IntegerExtension.register(&mut registry).unwrap();
    FloatExtension.register(&mut registry).unwrap();
    DoubleExtension.register(&mut registry).unwrap();

    assert_mixed_comparisons_are_registered(&registry);
}
