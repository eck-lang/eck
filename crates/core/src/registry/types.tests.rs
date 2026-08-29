use crate::{CoreError, Registry};

use super::super::test_support::{foreign_type_id, register_type, type_descriptor};

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
        registry.set_default_boolean(unknown),
        Err(CoreError::UnknownTypeId(id)) if id == unknown
    ));
}
