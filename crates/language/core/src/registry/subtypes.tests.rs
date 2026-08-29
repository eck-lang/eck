use crate::{
    BinaryOperator, CoreError, Registry, Scale, SubtypeBinaryRule, SubtypeDescriptor, ValueType,
};

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
