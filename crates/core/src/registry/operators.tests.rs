use super::*;

use super::super::test_support::{
    execute_context_operator, execute_in_place_operator, execute_operator, foreign_type_id,
    register_type,
};

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

/// Verifies context-aware registration stores both execution paths.
#[test]
fn context_operator_registration_stores_plain_and_context_callbacks() {
    let mut registry = Registry::new();
    let type_id = register_type(&mut registry, "int");

    let operator = registry
        .register_context_binary_operator(
            BinaryOperator::Addition,
            type_id,
            type_id,
            type_id,
            execute_operator,
            execute_context_operator,
        )
        .unwrap();
    let descriptor = registry.operator(operator).unwrap();

    assert!(descriptor.context_execute.is_some());
}

/// Verifies context-aware registration still rejects duplicate signatures.
#[test]
fn context_operator_registration_rejects_duplicate_signatures() {
    let mut registry = Registry::new();
    let type_id = register_type(&mut registry, "int");

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
        registry.register_context_binary_operator(
            BinaryOperator::Addition,
            type_id,
            type_id,
            type_id,
            execute_operator,
            execute_context_operator,
        ),
        Err(CoreError::DuplicateOperator { .. })
    ));
}

/// Verifies in-place registration augments an existing same-type descriptor.
#[test]
fn in_place_operator_registration_enables_the_optional_executor() {
    let mut registry = Registry::new();
    let type_id = register_type(&mut registry, "int");
    let operator = registry
        .register_binary_operator(
            BinaryOperator::Addition,
            type_id,
            type_id,
            type_id,
            execute_operator,
        )
        .unwrap();

    registry
        .register_in_place_binary_operator(
            BinaryOperator::Addition,
            type_id,
            type_id,
            execute_in_place_operator,
        )
        .unwrap();

    assert!(
        registry
            .operator(operator)
            .unwrap()
            .in_place_execute
            .is_some()
    );
}

/// Verifies in-place registration rejects a result type it cannot support.
#[test]
fn in_place_operator_registration_rejects_a_different_result_type() {
    let mut registry = Registry::new();
    let integer = register_type(&mut registry, "int");
    let decimal = register_type(&mut registry, "decimal");
    registry
        .register_binary_operator(
            BinaryOperator::Addition,
            integer,
            integer,
            decimal,
            execute_operator,
        )
        .unwrap();

    assert!(matches!(
        registry.register_in_place_binary_operator(
            BinaryOperator::Addition,
            integer,
            integer,
            execute_in_place_operator,
        ),
        Err(CoreError::InvalidInPlaceOperator {
            operator: BinaryOperator::Addition,
            ref left_operand_type,
            ref right_operand_type,
        }) if left_operand_type == "int" && right_operand_type == "int"
    ));
    assert!(
        registry
            .resolve_binary_operator(BinaryOperator::Addition, integer, integer)
            .is_ok()
    );
}
