use super::*;

use super::super::test_support::{
    evaluate_boolean, foreign_type_id, register_type, type_descriptor,
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
            .evaluate_boolean(&crate::Value::new(boolean, true))
            .unwrap()
    );
    assert!(matches!(
        registry.evaluate_boolean(&crate::Value::new(integer, 1_i64)),
        Err(CoreError::UnexpectedBooleanValueType { .. })
    ));
    assert!(matches!(
        registry.evaluate_boolean(&crate::Value::new(boolean, 1_i64)),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "test bool"
    ));
}
