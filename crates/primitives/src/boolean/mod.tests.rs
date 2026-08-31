use crate::DecimalExtension;
use crate::DoubleExtension;
use crate::FloatExtension;
use crate::IntegerExtension;
use language_core::{
    BinaryOperator, ComparisonOperator, CoreError, Extension, Registry, ValueType,
};

use super::BoolExtension;

/// Verifies that bool is used for unqualified boolean literals.
#[test]
fn bool_is_the_default_boolean_type() {
    let mut registry = Registry::new();
    BoolExtension.register(&mut registry).unwrap();

    let value = registry.parse_boolean("true", None).unwrap();

    assert_eq!(value.type_id(), registry.type_by_name("bool").unwrap());
    assert!(registry.evaluate_boolean(&value).unwrap());
    assert!(*value.downcast_ref::<bool>().unwrap());
}

/// Verifies that malformed source text is rejected as a boolean literal.
#[test]
fn bool_literals_reject_invalid_source_text() {
    let mut registry = Registry::new();
    BoolExtension.register(&mut registry).unwrap();

    let result = registry.parse_boolean("yes", None);

    assert!(matches!(result, Err(CoreError::InvalidLiteral { .. })));
}

/// Verifies that boolean defines equality but deliberately has no ordering.
#[test]
fn bool_registers_equality_comparisons_only() {
    let mut registry = Registry::new();
    BoolExtension.register(&mut registry).unwrap();
    let boolean = registry.type_by_name("bool").unwrap();

    for operator in [ComparisonOperator::Equal, ComparisonOperator::NotEqual] {
        assert!(
            registry
                .resolve_comparison(operator, boolean, boolean)
                .is_ok()
        );
    }
    for operator in [
        ComparisonOperator::Less,
        ComparisonOperator::LessOrEqual,
        ComparisonOperator::Greater,
        ComparisonOperator::GreaterOrEqual,
    ] {
        assert!(matches!(
            registry.resolve_comparison(operator, boolean, boolean),
            Err(CoreError::ComparisonNotDefined { .. })
        ));
    }
}

/// Verifies multiplication with every installed built-in numeric primitive.
#[test]
fn registers_multiplication_with_every_installed_numeric_primitive() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    FloatExtension.register(&mut registry).unwrap();
    DoubleExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    BoolExtension.register(&mut registry).unwrap();
    let boolean = registry.type_by_name("bool").unwrap();

    for numeric_name in ["int", "float", "double", "decimal"] {
        let numeric = registry.type_by_name(numeric_name).unwrap();
        for (left, right) in [(numeric, boolean), (boolean, numeric)] {
            assert_eq!(
                registry
                    .resolve_binary_operation(
                        BinaryOperator::Multiplication,
                        ValueType::plain(left),
                        ValueType::plain(right),
                    )
                    .unwrap()
                    .output,
                ValueType::plain(numeric)
            );
        }
    }
}
