use super::*;

use super::super::test_support::{
    foreign_subtype_id, foreign_type_id, register_subtype, register_type,
};

#[test]
fn subtype_registration_rejects_unallocated_and_duplicate_ids() {
    let mut registry = Registry::new();
    let unallocated = foreign_subtype_id();

    assert!(matches!(
        registry.register_subtype(SubtypeDescriptor {
            id: unallocated,
            name: "unallocated",
            suffixes: &["u"],
        }),
        Err(CoreError::UnallocatedSubtypeId(id)) if id == unallocated
    ));

    let id = register_subtype(&mut registry, "meter");
    assert!(matches!(
        registry.register_subtype(SubtypeDescriptor {
            id,
            name: "centimeter",
            suffixes: &["cm"],
        }),
        Err(CoreError::DuplicateSubtypeId(duplicate)) if duplicate == id
    ));
    assert_eq!(registry.subtype_by_name("meter"), Some(id));
    assert_eq!(registry.subtype_by_name("centimeter"), None);
}

#[test]
fn registered_subtype_ids_are_a_deterministic_snapshot() {
    let mut registry = Registry::new();
    let first = registry.allocate_subtype_id();
    let second = registry.allocate_subtype_id();
    let unregistered = registry.allocate_subtype_id();

    registry
        .register_subtype(SubtypeDescriptor {
            id: second,
            name: "second",
            suffixes: &["second"],
        })
        .unwrap();
    registry
        .register_subtype(SubtypeDescriptor {
            id: first,
            name: "first",
            suffixes: &["first"],
        })
        .unwrap();

    let registered_subtype_ids = registry.registered_subtype_ids();
    registry
        .register_subtype(SubtypeDescriptor {
            id: unregistered,
            name: "later",
            suffixes: &["later"],
        })
        .unwrap();
    let registered = registered_subtype_ids.collect::<Vec<_>>();

    assert_eq!(registered, vec![first, second]);
    assert!(!registered.contains(&unregistered));
}

#[test]
fn subtype_references_must_be_registered_before_use() {
    let mut registry = Registry::new();
    let registered = register_subtype(&mut registry, "meter");
    let unknown = foreign_subtype_id();

    assert!(matches!(
        registry.register_subtype_binary_rule(
            BinaryOperator::Addition,
            Some(registered),
            Some(registered),
            SubtypeBinaryRule::new(Some(unknown)),
        ),
        Err(CoreError::UnknownSubtypeId(id)) if id == unknown
    ));
    assert!(matches!(
        registry.register_subtype_binary_rule(
            BinaryOperator::Addition,
            Some(unknown),
            None,
            SubtypeBinaryRule::new(None),
        ),
        Err(CoreError::UnknownSubtypeId(id)) if id == unknown
    ));
    assert!(matches!(
        registry.register_subtype_conversion(registered, unknown, Scale::IDENTITY),
        Err(CoreError::UnknownSubtypeId(id)) if id == unknown
    ));
}

#[test]
fn subtype_rules_require_at_least_one_qualified_operand() {
    let mut registry = Registry::new();

    assert!(matches!(
        registry.register_subtype_binary_rule(
            BinaryOperator::Addition,
            None,
            None,
            SubtypeBinaryRule::new(None),
        ),
        Err(CoreError::UnreachableSubtypeOperatorRule(
            BinaryOperator::Addition
        ))
    ));
}

#[test]
fn subtype_conversions_reject_invalid_identity_scales() {
    let mut registry = Registry::new();
    let subtype = register_subtype(&mut registry, "meter");

    assert!(matches!(
        registry.register_subtype_conversion(subtype, subtype, Scale::integer(2)),
        Err(CoreError::InvalidIdentitySubtypeConversion(name)) if name == "meter"
    ));
}

#[test]
fn subtype_conversion_resolution_rejects_unknown_references() {
    let mut registry = Registry::new();
    let type_id = register_type(&mut registry, "int");
    let subtype = register_subtype(&mut registry, "meter");
    let unknown_type = foreign_type_id();
    let unknown_subtype = foreign_subtype_id();

    assert!(matches!(
        registry.resolve_subtype_conversion(ValueType::qualified(unknown_type, subtype), subtype),
        Err(CoreError::UnknownTypeId(id)) if id == unknown_type
    ));
    assert!(matches!(
        registry.resolve_subtype_conversion(ValueType::qualified(type_id, unknown_subtype), subtype),
        Err(CoreError::UnknownSubtypeId(id)) if id == unknown_subtype
    ));
    assert!(matches!(
        registry.resolve_subtype_conversion(ValueType::qualified(type_id, subtype), unknown_subtype),
        Err(CoreError::UnknownSubtypeId(id)) if id == unknown_subtype
    ));
}

#[test]
fn relative_rules_reject_invalid_registrations() {
    let mut registry = Registry::new();
    let percentage = register_subtype(&mut registry, "percentage");
    let unknown = foreign_subtype_id();

    assert!(matches!(
        registry.register_subtype_relative_rule(
            BinaryOperator::Multiplication,
            None,
            Some(percentage),
            SubtypeRelativeRule::new(Scale::new(1, 100)),
        ),
        Err(CoreError::InvalidRelativeOperator(
            BinaryOperator::Multiplication
        ))
    ));
    assert!(matches!(
        registry.register_subtype_relative_rule(
            BinaryOperator::Addition,
            None,
            None,
            SubtypeRelativeRule::new(Scale::new(1, 100)),
        ),
        Err(CoreError::UnreachableSubtypeRelativeRule(
            BinaryOperator::Addition
        ))
    ));
    assert!(matches!(
        registry.register_subtype_relative_rule(
            BinaryOperator::Subtraction,
            None,
            Some(percentage),
            SubtypeRelativeRule::new(Scale::new(1, 0)),
        ),
        Err(CoreError::InvalidScale)
    ));
    assert!(matches!(
        registry.register_subtype_relative_rule(
            BinaryOperator::Addition,
            None,
            Some(unknown),
            SubtypeRelativeRule::new(Scale::new(1, 100)),
        ),
        Err(CoreError::UnknownSubtypeId(id)) if id == unknown
    ));
}

#[test]
fn relative_rules_require_a_qualified_right_operand() {
    let mut registry = Registry::new();
    let unit = register_subtype(&mut registry, "meter");

    assert!(matches!(
        registry.register_subtype_relative_rule(
            BinaryOperator::Addition,
            Some(unit),
            None,
            SubtypeRelativeRule::new(Scale::new(1, 100)),
        ),
        Err(CoreError::RelativeRuleRequiresQualifiedRightOperand(
            BinaryOperator::Addition
        ))
    ));
}

#[test]
fn relative_rules_reject_duplicates() {
    let mut registry = Registry::new();
    let percentage = register_subtype(&mut registry, "percentage");
    let rule = SubtypeRelativeRule::new(Scale::new(1, 100));

    registry
        .register_subtype_relative_rule(BinaryOperator::Addition, None, Some(percentage), rule)
        .unwrap();
    assert!(matches!(
        registry.register_subtype_relative_rule(BinaryOperator::Addition, None, Some(percentage), rule),
        Err(CoreError::DuplicateSubtypeRelativeOperator { operator, .. })
            if operator == BinaryOperator::Addition
    ));
}

#[test]
fn relative_resolution_computes_the_adjusted_output_type() {
    let mut registry = Registry::new();
    let integer = register_type(&mut registry, "int");
    let fractional = register_type(&mut registry, "decimal");
    registry.set_default_integer(integer).unwrap();
    registry.set_default_fractional(fractional).unwrap();
    let percentage = register_subtype(&mut registry, "percentage");

    for operator in [
        BinaryOperator::Division,
        BinaryOperator::Multiplication,
        BinaryOperator::Addition,
        BinaryOperator::Subtraction,
    ] {
        registry
            .register_binary_operator(
                operator,
                integer,
                fractional,
                fractional,
                super::super::test_support::execute_operator,
            )
            .unwrap();
    }
    registry
        .register_subtype_relative_rule(
            BinaryOperator::Subtraction,
            None,
            Some(percentage),
            SubtypeRelativeRule::new(Scale::new(1, 100)),
        )
        .unwrap();

    let resolution = registry
        .resolve_subtype_relative_rule(
            BinaryOperator::Subtraction,
            ValueType::plain(integer),
            ValueType::qualified(integer, percentage),
        )
        .unwrap();

    assert_eq!(resolution.output, ValueType::plain(fractional));
    assert_eq!(resolution.left_operand_scale, Scale::IDENTITY);
    assert_eq!(resolution.right_operand_scale, Scale::new(1, 100));
    assert_eq!(resolution.relative_adjustment, Some(Scale::new(1, 100)));
}

#[test]
fn relative_resolution_preserves_the_left_operand_subtype() {
    let mut registry = Registry::new();
    let integer = register_type(&mut registry, "int");
    let fractional = register_type(&mut registry, "decimal");
    registry.set_default_integer(integer).unwrap();
    registry.set_default_fractional(fractional).unwrap();
    let unit = register_subtype(&mut registry, "meter");
    let percentage = registry.allocate_subtype_id();
    registry
        .register_subtype(SubtypeDescriptor {
            id: percentage,
            name: "percentage",
            suffixes: &["%"],
        })
        .unwrap();

    for operator in [
        BinaryOperator::Division,
        BinaryOperator::Multiplication,
        BinaryOperator::Addition,
    ] {
        registry
            .register_binary_operator(
                operator,
                fractional,
                fractional,
                fractional,
                super::super::test_support::execute_operator,
            )
            .unwrap();
    }
    registry
        .register_subtype_relative_rule(
            BinaryOperator::Addition,
            Some(unit),
            Some(percentage),
            SubtypeRelativeRule::new(Scale::new(1, 100)),
        )
        .unwrap();

    let resolution = registry
        .resolve_subtype_relative_rule(
            BinaryOperator::Addition,
            ValueType::qualified(fractional, unit),
            ValueType::qualified(fractional, percentage),
        )
        .unwrap();

    assert_eq!(resolution.output, ValueType::qualified(fractional, unit));
    assert_eq!(resolution.relative_adjustment, Some(Scale::new(1, 100)));
}

#[test]
fn relative_resolution_reports_missing_rules() {
    let mut registry = Registry::new();
    let type_id = register_type(&mut registry, "int");
    let subtype = register_subtype(&mut registry, "percentage");

    assert!(matches!(
        registry.resolve_subtype_relative_rule(
            BinaryOperator::Addition,
            ValueType::plain(type_id),
            ValueType::qualified(type_id, subtype),
        ),
        Err(CoreError::SubtypeRelativeOperatorNotDefined { operator, .. })
            if operator == BinaryOperator::Addition
    ));
}

#[test]
fn fractional_subtype_scales_resolve_the_promoted_operator() {
    let mut registry = Registry::new();
    let integer = register_type(&mut registry, "int");
    let fractional = register_type(&mut registry, "decimal");
    registry.set_default_integer(integer).unwrap();
    registry.set_default_fractional(fractional).unwrap();
    let percentage = register_subtype(&mut registry, "percentage");

    registry
        .register_binary_operator(
            BinaryOperator::Division,
            integer,
            fractional,
            fractional,
            super::super::test_support::execute_operator,
        )
        .unwrap();
    registry
        .register_binary_operator(
            BinaryOperator::Multiplication,
            integer,
            fractional,
            fractional,
            super::super::test_support::execute_operator,
        )
        .unwrap();
    registry
        .register_subtype_binary_rule(
            BinaryOperator::Multiplication,
            None,
            Some(percentage),
            SubtypeBinaryRule::new(None).with_operand_scales(Scale::IDENTITY, Scale::new(1, 100)),
        )
        .unwrap();

    let resolution = registry
        .resolve_binary_operation(
            BinaryOperator::Multiplication,
            ValueType::plain(integer),
            ValueType::qualified(integer, percentage),
        )
        .unwrap();

    let descriptor = registry.operator(resolution.operator).unwrap();
    assert_eq!(descriptor.left_operand_type, integer);
    assert_eq!(descriptor.right_operand_type, fractional);
    assert_eq!(resolution.output, ValueType::plain(fractional));
}

#[test]
fn fractional_subtype_scales_require_the_promoted_division_operator() {
    let mut registry = Registry::new();
    let integer = register_type(&mut registry, "int");
    let fractional = register_type(&mut registry, "decimal");
    registry.set_default_integer(integer).unwrap();
    registry.set_default_fractional(fractional).unwrap();
    let percentage = register_subtype(&mut registry, "percentage");

    registry
        .register_binary_operator(
            BinaryOperator::Multiplication,
            integer,
            integer,
            integer,
            super::super::test_support::execute_operator,
        )
        .unwrap();
    registry
        .register_subtype_binary_rule(
            BinaryOperator::Multiplication,
            None,
            Some(percentage),
            SubtypeBinaryRule::new(None).with_operand_scales(Scale::IDENTITY, Scale::new(1, 100)),
        )
        .unwrap();

    assert!(matches!(
        registry.resolve_binary_operation(
            BinaryOperator::Multiplication,
            ValueType::plain(integer),
            ValueType::qualified(integer, percentage),
        ),
        Err(CoreError::OperatorNotDefined {
            operator: BinaryOperator::Division,
            left_operand_type,
            right_operand_type,
        }) if left_operand_type == "int" && right_operand_type == "decimal"
    ));
}
