use crate::{
    BinaryOperator, CoreError, FunctionSignature, Registry, Scale, SubtypeBinaryRule,
    SubtypeDescriptor, SubtypeId, TypeDescriptor, TypeId, Value,
};

fn format_value(_: &Value) -> Result<String, CoreError> {
    Ok(String::new())
}

fn execute_operator(left_operand: &Value, _: &Value) -> Result<Value, CoreError> {
    Ok(left_operand.clone())
}

fn execute_function(_: &Registry, _: &[Value]) -> Result<Option<Value>, CoreError> {
    Ok(None)
}

fn type_descriptor(id: TypeId, name: &'static str) -> TypeDescriptor {
    TypeDescriptor {
        id,
        name,
        parse_numeric_literal: None,
        parse_string_literal: None,
        format: format_value,
    }
}

fn register_type(registry: &mut Registry, name: &'static str) -> TypeId {
    let id = registry.allocate_type_id();
    registry.register_type(type_descriptor(id, name)).unwrap();
    id
}

fn register_subtype(registry: &mut Registry, name: &'static str) -> SubtypeId {
    let id = registry.allocate_subtype_id();
    registry
        .register_subtype(SubtypeDescriptor {
            id,
            name,
            suffixes: &["unit"],
        })
        .unwrap();
    id
}

fn foreign_type_id() -> TypeId {
    Registry::new().allocate_type_id()
}

fn foreign_subtype_id() -> SubtypeId {
    Registry::new().allocate_subtype_id()
}

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
fn type_references_must_be_registered_before_use() {
    let mut registry = Registry::new();
    let registered = register_type(&mut registry, "int");
    let unknown = foreign_type_id();

    assert!(matches!(
        registry.set_default_integer(unknown),
        Err(CoreError::UnknownTypeId(id)) if id == unknown
    ));
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
    assert!(matches!(
        registry.register_function(
            "identity",
            FunctionSignature::Exact(vec![registered, unknown]),
            None,
            execute_function,
        ),
        Err(CoreError::UnknownTypeId(id)) if id == unknown
    ));
    assert!(matches!(
        registry.register_function(
            "make_unknown",
            FunctionSignature::Exact(vec![registered]),
            Some(unknown),
            execute_function,
        ),
        Err(CoreError::UnknownTypeId(id)) if id == unknown
    ));
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
        registry.resolve_subtype_conversion(
            crate::ValueType::qualified(unknown_type, subtype),
            subtype,
        ),
        Err(CoreError::UnknownTypeId(id)) if id == unknown_type
    ));
    assert!(matches!(
        registry.resolve_subtype_conversion(
            crate::ValueType::qualified(type_id, unknown_subtype),
            subtype,
        ),
        Err(CoreError::UnknownSubtypeId(id)) if id == unknown_subtype
    ));
    assert!(matches!(
        registry.resolve_subtype_conversion(
            crate::ValueType::qualified(type_id, subtype),
            unknown_subtype,
        ),
        Err(CoreError::UnknownSubtypeId(id)) if id == unknown_subtype
    ));
}

#[test]
fn executable_descriptor_ids_are_scoped_to_their_registry() {
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
    registry
        .register_function(
            "identity",
            FunctionSignature::Exact(vec![type_id]),
            Some(type_id),
            execute_function,
        )
        .unwrap();

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
    let foreign_function = foreign_registry
        .register_function(
            "identity",
            FunctionSignature::Exact(vec![foreign_type_id]),
            Some(foreign_type_id),
            execute_function,
        )
        .unwrap();

    assert!(matches!(
        registry.operator(foreign_operator),
        Err(CoreError::UnknownOperatorId(id)) if id == foreign_operator
    ));
    assert!(matches!(
        registry.function(foreign_function),
        Err(CoreError::UnknownFunctionId(id)) if id == foreign_function
    ));
}

#[test]
fn function_overloads_are_deterministic_and_unique() {
    for register_fallback_first in [true, false] {
        let mut registry = Registry::new();
        let type_id = register_type(&mut registry, "int");
        let exact_signature = FunctionSignature::Exact(vec![type_id]);

        if register_fallback_first {
            registry
                .register_function(
                    "identity",
                    FunctionSignature::AnySingle,
                    Some(type_id),
                    execute_function,
                )
                .unwrap();
        }
        registry
            .register_function(
                "identity",
                exact_signature.clone(),
                Some(type_id),
                execute_function,
            )
            .unwrap();
        if !register_fallback_first {
            registry
                .register_function(
                    "identity",
                    FunctionSignature::AnySingle,
                    Some(type_id),
                    execute_function,
                )
                .unwrap();
        }

        let resolved = registry.resolve_function("identity", &[type_id]).unwrap();
        assert!(matches!(
            &registry.function(resolved).unwrap().signature,
            FunctionSignature::Exact(types) if types == &[type_id]
        ));

        assert!(matches!(
            registry.register_function(
                "identity",
                exact_signature,
                Some(type_id),
                execute_function,
            ),
            Err(CoreError::DuplicateFunctionSignature { name, signature })
                if name == "identity" && signature == "(int)"
        ));
        assert!(matches!(
            registry.register_function(
                "identity",
                FunctionSignature::AnySingle,
                Some(type_id),
                execute_function,
            ),
            Err(CoreError::DuplicateFunctionSignature { name, signature })
                if name == "identity" && signature == "(any)"
        ));
    }
}
