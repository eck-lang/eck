use super::*;
use float::FloatExtension;
use integer::IntegerExtension;
use language_core::{BinaryOperator, ComparisonOperator, Extension, Registry, ValueType};

/// Verifies every double comparison relation in one populated registry.
fn assert_comparisons_are_registered(registry: &Registry) {
    let double = registry.type_by_name("double").unwrap();

    for other_type_name in ["double", "float", "int"] {
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
                    .resolve_comparison(operator, double, other_type)
                    .is_ok()
            );
            assert!(
                registry
                    .resolve_comparison(operator, other_type, double)
                    .is_ok()
            );
        }
    }
}

/// Verifies that double literals retain their precision and native formatting.
#[test]
fn double_literals_preserve_double_precision_and_native_formatting() {
    let mut registry = Registry::new();
    DoubleExtension.register(&mut registry).unwrap();
    let double = registry.type_by_name("double").unwrap();

    let value = registry.parse_numeric("16777217", Some(double)).unwrap();

    assert_eq!(*value.downcast_ref::<f64>().unwrap(), 16_777_217.0);
    assert_eq!(registry.format_value(&value).unwrap(), "16777217");
}

/// Verifies that malformed source text is rejected as a double literal.
#[test]
fn double_literals_reject_invalid_source_text() {
    let mut registry = Registry::new();
    DoubleExtension.register(&mut registry).unwrap();
    let double = registry.type_by_name("double").unwrap();

    let result = registry.parse_numeric("not-a-number", Some(double));

    assert!(matches!(result, Err(CoreError::InvalidLiteral { .. })));
}

/// Verifies all arithmetic operators that losslessly promote float to double.
#[test]
fn double_registers_all_lossless_float_promotion_operators() {
    let mut registry = Registry::new();
    FloatExtension.register(&mut registry).unwrap();
    DoubleExtension.register(&mut registry).unwrap();

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
                    ValueType::plain(float),
                    ValueType::plain(double),
                )
                .unwrap()
                .output,
            ValueType::plain(double)
        );
        assert_eq!(
            registry
                .resolve_binary_operation(
                    operator,
                    ValueType::plain(double),
                    ValueType::plain(float),
                )
                .unwrap()
                .output,
            ValueType::plain(double)
        );
    }
}

/// Verifies all arithmetic operators that convert integer operands to double.
#[test]
fn double_registers_all_integer_promotion_operators() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DoubleExtension.register(&mut registry).unwrap();

    let integer = registry.type_by_name("int").unwrap();
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
                    ValueType::plain(integer),
                    ValueType::plain(double),
                )
                .unwrap()
                .output,
            ValueType::plain(double)
        );
        assert_eq!(
            registry
                .resolve_binary_operation(
                    operator,
                    ValueType::plain(double),
                    ValueType::plain(integer),
                )
                .unwrap()
                .output,
            ValueType::plain(double)
        );
    }
}

/// Verifies comparisons when float and integer are registered before double.
#[test]
fn double_registers_comparisons_when_other_types_are_registered_first() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    FloatExtension.register(&mut registry).unwrap();
    DoubleExtension.register(&mut registry).unwrap();

    assert_comparisons_are_registered(&registry);
}

/// Verifies comparisons when float and integer are registered after double.
#[test]
fn double_registers_comparisons_when_other_types_are_registered_later() {
    let mut registry = Registry::new();
    DoubleExtension.register(&mut registry).unwrap();
    IntegerExtension.register(&mut registry).unwrap();
    FloatExtension.register(&mut registry).unwrap();

    assert_comparisons_are_registered(&registry);
}
