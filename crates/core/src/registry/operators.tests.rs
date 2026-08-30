use super::*;

use super::super::test_support::{execute_operator, foreign_type_id, register_type};

#[test]
fn operator_types_must_be_registered_before_use() {
    let mut registry = Registry::new();
    let registered = register_type(&mut registry, "int");
    let unknown = foreign_type_id();

    assert!(matches!(
        registry.register_binary_operator(
            BinaryOperator::Addition,
            registered,
            registered,
            unknown,
            execute_operator,
        ),
        Err(CoreError::UnknownTypeId(id)) if id == unknown
    ));
}

#[test]
fn operator_descriptor_ids_are_scoped_to_their_registry() {
    let mut registry = Registry::new();
    let type_id = register_type(&mut registry, "int");

    let mut foreign_registry = Registry::new();
    let foreign_type_id = register_type(&mut foreign_registry, "int");
    let foreign_operator = foreign_registry
        .register_binary_operator(
            BinaryOperator::Addition,
            foreign_type_id,
            foreign_type_id,
            foreign_type_id,
            execute_operator,
        )
        .unwrap();

    registry
        .register_binary_operator(
            BinaryOperator::Addition,
            type_id,
            type_id,
            type_id,
            execute_operator,
        )
        .unwrap();

    assert!(matches!(
        registry.operator(foreign_operator),
        Err(CoreError::UnknownOperatorId(id)) if id == foreign_operator
    ));
}
