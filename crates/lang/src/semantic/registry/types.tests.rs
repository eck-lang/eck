use super::*;

use super::super::test_support::{
    evaluate_boolean, foreign_subtype_id, foreign_type_id, register_subtype, register_type,
    type_descriptor,
};

#[test]
fn type_registration_rejects_unallocated_and_duplicate_ids() {
    let mut registry = Registry::new();
    let unallocated = foreign_type_id();

    assert!(matches!(
        registry.register_type(type_descriptor(unallocated, "unallocated")),
        Err(CoreError::UnallocatedTypeId(id)) if id == unallocated
    ));

    let id = register_type(&mut registry, "int");
    assert!(matches!(
        registry.register_type(type_descriptor(id, "decimal")),
        Err(CoreError::DuplicateTypeId(duplicate)) if duplicate == id
    ));
    assert_eq!(registry.type_by_name("int"), Some(id));
    assert_eq!(registry.type_by_name("decimal"), None);
    assert_eq!(registry.type_name(id), "int");
}

#[test]
fn default_types_must_be_registered_before_use() {
    let mut registry = Registry::new();
    let unknown = foreign_type_id();

    assert!(matches!(
        registry.set_default_integer(unknown),
        Err(CoreError::UnknownTypeId(id)) if id == unknown
    ));
    assert!(matches!(
        registry.set_default_fractional(unknown),
        Err(CoreError::UnknownTypeId(id)) if id == unknown
    ));
    assert!(matches!(
        registry.set_default_string(unknown),
        Err(CoreError::UnknownTypeId(id)) if id == unknown
    ));
    assert!(matches!(
        registry.set_default_boolean(unknown, evaluate_boolean),
        Err(CoreError::UnknownTypeId(id)) if id == unknown
    ));
}

#[test]
fn default_boolean_evaluation_uses_the_registered_type_contract() {
    let mut registry = Registry::new();
    let boolean = register_type(&mut registry, "bool");
    let integer = register_type(&mut registry, "int");
    registry
        .set_default_boolean(boolean, evaluate_boolean)
        .unwrap();

    assert!(
        registry
            .evaluate_boolean(&crate::values::Value::new(boolean, true))
            .unwrap()
    );
    assert!(matches!(
        registry.evaluate_boolean(&crate::values::Value::new(integer, 1_i64)),
        Err(CoreError::UnexpectedBooleanValueType { .. })
    ));
    assert!(matches!(
        registry.evaluate_boolean(&crate::values::Value::new(boolean, 1_i64)),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "test bool"
    ));
}

/// Verifies integer-only language features can use an explicit type capability.
#[test]
fn reports_the_registered_integer_capability() {
    let mut registry = Registry::new();
    let integer = registry.allocate_type_id();
    let fractional = registry.allocate_type_id();
    let mut integer_descriptor = type_descriptor(integer, "int");
    integer_descriptor.is_integer = true;

    registry.register_type(integer_descriptor).unwrap();
    registry
        .register_type(type_descriptor(fractional, "decimal"))
        .unwrap();

    assert!(registry.is_integer_type(integer).unwrap());
    assert!(!registry.is_integer_type(fractional).unwrap());
    assert!(matches!(
        registry.is_integer_type(foreign_type_id()),
        Err(CoreError::UnknownTypeId(_))
    ));
}

/// Verifies validation accepts registered capabilities and rejects unknown ones.
#[test]
fn value_validation_rejects_unregistered_types_and_subtypes() {
    let mut registry = Registry::new();
    let integer = register_type(&mut registry, "int");
    let subtype_id = register_subtype(&mut registry, "unit");

    let plain = crate::values::Value::new(integer, 1_i64);
    let qualified = crate::values::Value::new(integer, 1_i64).with_subtype(Some(subtype_id));
    assert!(registry.validate_value(&plain).is_ok());
    assert!(registry.validate_value(&qualified).is_ok());

    let unknown_type = crate::values::Value::new(foreign_type_id(), 1_i64);
    assert!(matches!(
        registry.validate_value(&unknown_type),
        Err(CoreError::UnknownTypeId(_))
    ));

    let unknown_subtype =
        crate::values::Value::new(integer, 1_i64).with_subtype(Some(foreign_subtype_id()));
    assert!(matches!(
        registry.validate_value(&unknown_subtype),
        Err(CoreError::UnknownSubtypeId(_))
    ));
}
