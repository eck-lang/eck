use super::*;
use crate::Scale;

use super::super::test_support::{
    foreign_subtype_id, foreign_type_id, register_subtype, register_type,
};

fn execute_comparison(_: &crate::Value, _: &crate::Value) -> Result<bool, CoreError> {
    Ok(true)
}

#[test]
fn registers_and_resolves_exact_comparisons_without_a_boolean_type() {
    let mut registry = Registry::new();
    let integer = register_type(&mut registry, "int");
    let comparison = registry
        .register_comparison(
            ComparisonOperator::Equal,
            integer,
            integer,
            execute_comparison,
        )
        .unwrap();

    assert_eq!(
        registry
            .resolve_comparison(ComparisonOperator::Equal, integer, integer)
            .unwrap(),
        comparison
    );
}

#[test]
fn rejects_invalid_and_duplicate_comparison_registrations() {
    let mut registry = Registry::new();
    let integer = register_type(&mut registry, "int");
    let foreign = foreign_type_id();
    assert!(matches!(
        registry.register_comparison(ComparisonOperator::Equal, integer, foreign, execute_comparison),
        Err(CoreError::UnknownTypeId(id)) if id == foreign
    ));
    registry
        .register_comparison(
            ComparisonOperator::Equal,
            integer,
            integer,
            execute_comparison,
        )
        .unwrap();
    assert!(matches!(
        registry.register_comparison(
            ComparisonOperator::Equal,
            integer,
            integer,
            execute_comparison
        ),
        Err(CoreError::DuplicateComparison { .. })
    ));
}

#[test]
fn named_comparisons_activate_independently_of_type_registration_order() {
    let mut decimal_first = Registry::new();
    let decimal = register_type(&mut decimal_first, "decimal");
    decimal_first
        .declare_comparison(
            ComparisonOperator::Equal,
            "decimal",
            "float",
            execute_comparison,
        )
        .unwrap();
    let float = register_type(&mut decimal_first, "float");

    assert!(
        decimal_first
            .resolve_comparison(ComparisonOperator::Equal, decimal, float)
            .is_ok()
    );

    let mut float_first = Registry::new();
    let float = register_type(&mut float_first, "float");
    let decimal = register_type(&mut float_first, "decimal");
    float_first
        .declare_comparison(
            ComparisonOperator::Equal,
            "decimal",
            "float",
            execute_comparison,
        )
        .unwrap();

    assert!(
        float_first
            .resolve_comparison(ComparisonOperator::Equal, decimal, float)
            .is_ok()
    );
}

#[test]
fn named_comparisons_reject_duplicate_declarations() {
    let mut registry = Registry::new();
    registry
        .declare_comparison(
            ComparisonOperator::Equal,
            "decimal",
            "float",
            execute_comparison,
        )
        .unwrap();

    assert!(matches!(
        registry.declare_comparison(
            ComparisonOperator::Equal,
            "decimal",
            "float",
            execute_comparison,
        ),
        Err(CoreError::DuplicateComparison { .. })
    ));
}

#[test]
fn comparison_descriptor_ids_are_scoped_to_their_registry() {
    let mut registry = Registry::new();
    let integer = register_type(&mut registry, "int");

    let mut foreign_registry = Registry::new();
    let foreign_integer = register_type(&mut foreign_registry, "int");
    let foreign_comparison = foreign_registry
        .register_comparison(
            ComparisonOperator::Equal,
            foreign_integer,
            foreign_integer,
            execute_comparison,
        )
        .unwrap();

    registry
        .register_comparison(
            ComparisonOperator::Equal,
            integer,
            integer,
            execute_comparison,
        )
        .unwrap();

    assert!(matches!(
        registry.comparison(foreign_comparison),
        Err(CoreError::UnknownComparisonId(id)) if id == foreign_comparison
    ));
}

#[test]
fn subtype_comparison_rules_validate_their_registration() {
    let mut registry = Registry::new();
    let meter = register_subtype(&mut registry, "meter");
    let foreign = foreign_subtype_id();

    assert!(matches!(
        registry.register_subtype_comparison_rule(
            ComparisonOperator::Equal,
            None,
            None,
            SubtypeComparisonRule::new(),
        ),
        Err(CoreError::UnreachableSubtypeComparisonRule(
            ComparisonOperator::Equal
        ))
    ));
    assert!(matches!(
        registry.register_subtype_comparison_rule(
            ComparisonOperator::Equal,
            Some(meter),
            Some(foreign),
            SubtypeComparisonRule::new(),
        ),
        Err(CoreError::UnknownSubtypeId(id)) if id == foreign
    ));
    assert!(matches!(
        registry.register_subtype_comparison_rule(
            ComparisonOperator::Equal,
            Some(meter),
            None,
            SubtypeComparisonRule::new().with_operand_scales(Scale::new(1, 0), Scale::IDENTITY),
        ),
        Err(CoreError::InvalidScale)
    ));

    registry
        .register_subtype_comparison_rule(
            ComparisonOperator::Equal,
            Some(meter),
            None,
            SubtypeComparisonRule::new(),
        )
        .unwrap();
    assert!(matches!(
        registry.register_subtype_comparison_rule(
            ComparisonOperator::Equal,
            Some(meter),
            None,
            SubtypeComparisonRule::new(),
        ),
        Err(CoreError::DuplicateSubtypeComparison { .. })
    ));
}

#[test]
fn qualified_comparisons_apply_scales_and_return_plain_boolean() {
    let mut registry = Registry::new();
    let integer = register_type(&mut registry, "int");
    let boolean = register_type(&mut registry, "bool");
    registry
        .set_default_boolean(boolean, super::super::test_support::evaluate_boolean)
        .unwrap();
    let meter = register_subtype(&mut registry, "meter");
    let centimeter = registry.allocate_subtype_id();
    registry
        .register_subtype(crate::SubtypeDescriptor {
            id: centimeter,
            name: "centimeter",
            suffixes: &["centimeter"],
        })
        .unwrap();
    registry
        .register_comparison(
            ComparisonOperator::Equal,
            integer,
            integer,
            execute_comparison,
        )
        .unwrap();
    registry
        .register_binary_operator(
            crate::BinaryOperator::Multiplication,
            integer,
            integer,
            integer,
            super::super::test_support::execute_operator,
        )
        .unwrap();
    registry
        .register_subtype_comparison_rule(
            ComparisonOperator::Equal,
            Some(meter),
            Some(centimeter),
            SubtypeComparisonRule::new().with_operand_scales(Scale::integer(100), Scale::IDENTITY),
        )
        .unwrap();

    let resolved = registry
        .resolve_comparison_operation(
            ComparisonOperator::Equal,
            ValueType::qualified(integer, meter),
            ValueType::qualified(integer, centimeter),
        )
        .unwrap();

    assert_eq!(resolved.output, ValueType::plain(boolean));
    assert_eq!(resolved.left_operand_scale, Scale::integer(100));
}

#[test]
fn qualified_comparisons_require_an_exact_subtype_rule() {
    let mut registry = Registry::new();
    let integer = register_type(&mut registry, "int");
    let boolean = register_type(&mut registry, "bool");
    registry
        .set_default_boolean(boolean, super::super::test_support::evaluate_boolean)
        .unwrap();
    let meter = register_subtype(&mut registry, "meter");

    assert!(matches!(
        registry.resolve_comparison_operation(
            ComparisonOperator::Less,
            ValueType::qualified(integer, meter),
            ValueType::plain(integer),
        ),
        Err(CoreError::SubtypeComparisonNotDefined { .. })
    ));
}
