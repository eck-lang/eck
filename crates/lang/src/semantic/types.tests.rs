//! Unit tests for the structural semantic types shared across compiler stages.

use super::*;
use crate::ScalarRepresentation;

/// Verifies that an array semantic type retains its element representation mode.
#[test]
fn array_semantic_type_retains_element_representation() {
    let type_id = crate::Registry::new().allocate_type_id();
    let array_type = ArrayType::static_element(
        SemanticType::Scalar(ValueType::plain(type_id)),
        ScalarRepresentation::AdaptiveSignedInteger,
    );

    assert_eq!(
        SemanticType::array(array_type.clone()),
        SemanticType::array(array_type.clone())
    );
    assert_eq!(
        array_type.static_representation(),
        Some(ScalarRepresentation::AdaptiveSignedInteger)
    );
}

/// Verifies that scalar and array shapes remain distinct semantic contracts.
#[test]
fn scalar_and_array_semantic_types_are_distinct() {
    let type_id = crate::Registry::new().allocate_type_id();
    let scalar_type = SemanticType::Scalar(ValueType::plain(type_id));
    let array_type = SemanticType::array(ArrayType::static_element(
        SemanticType::Scalar(ValueType::plain(type_id)),
        ScalarRepresentation::Exact,
    ));

    assert_ne!(scalar_type, array_type);
}

/// Verifies unions flatten, deduplicate, and compare independently of source order.
#[test]
fn unions_are_canonical_structural_sets() {
    let mut registry = crate::Registry::new();
    let integer = registry.allocate_type_id();
    let string = registry.allocate_type_id();
    let decimal = registry.allocate_type_id();
    let integer = SemanticType::Scalar(ValueType::plain(integer));
    let string = SemanticType::Scalar(ValueType::plain(string));
    let decimal = SemanticType::Scalar(ValueType::plain(decimal));

    let nested = SemanticType::union([
        integer.clone(),
        SemanticType::union([string.clone(), integer.clone()]),
        decimal.clone(),
    ]);
    let reordered = SemanticType::union([decimal, string, integer]);

    assert_eq!(nested, reordered);
    assert!(matches!(nested, SemanticType::Union(members) if members.len() == 3));
}

/// Verifies duplicate-only unions collapse without changing array structure.
#[test]
fn unions_collapse_single_members_without_distributing_arrays() {
    let mut registry = crate::Registry::new();
    let integer = SemanticType::Scalar(ValueType::plain(registry.allocate_type_id()));
    let collapsed = SemanticType::union([integer.clone(), integer.clone()]);
    let array = SemanticType::array(ArrayType::static_element(
        integer.clone(),
        ScalarRepresentation::Exact,
    ));
    let union_of_array_and_scalar = SemanticType::union([array.clone(), integer.clone()]);

    assert_eq!(collapsed, integer);
    assert!(matches!(union_of_array_and_scalar, SemanticType::Union(_)));
    assert_ne!(union_of_array_and_scalar, array);
}

/// Verifies union membership rules and invariant mutable-array contracts.
#[test]
fn assignability_keeps_union_and_array_shapes_distinct() {
    let mut registry = crate::Registry::new();
    let integer = registry.allocate_type_id();
    let string = registry.allocate_type_id();
    let integer = SemanticType::Scalar(ValueType::plain(integer));
    let string = SemanticType::Scalar(ValueType::plain(string));
    let scalar_union = SemanticType::union([integer.clone(), string.clone()]);
    let int_array = SemanticType::array(ArrayType::static_element(
        integer.clone(),
        ScalarRepresentation::AdaptiveSignedInteger,
    ));
    let string_array = SemanticType::array(ArrayType::static_element(
        string.clone(),
        ScalarRepresentation::Exact,
    ));
    let union_element_array = SemanticType::array(ArrayType::static_element(
        scalar_union.clone(),
        ScalarRepresentation::AdaptiveSignedInteger,
    ));
    let array_union = SemanticType::union([int_array.clone(), string_array]);
    let null = SemanticType::Scalar(ValueType::plain(registry.allocate_type_id()));
    let nullable_int = SemanticType::union([integer.clone(), null]);

    assert!(is_assignable(&integer, &scalar_union));
    assert!(!is_assignable(&scalar_union, &integer));
    assert!(is_assignable(&int_array, &array_union));
    assert!(!is_assignable(&int_array, &union_element_array));
    assert!(is_assignable(&nullable_int, &nullable_int));
    assert!(is_assignable(&integer, &nullable_int));
    assert!(!is_assignable(&nullable_int, &integer));
}

/// Verifies union-to-union assignment follows structural set inclusion.
#[test]
fn union_assignability_requires_every_source_member() {
    let mut registry = crate::Registry::new();
    let integer = SemanticType::Scalar(ValueType::plain(registry.allocate_type_id()));
    let string = SemanticType::Scalar(ValueType::plain(registry.allocate_type_id()));
    let decimal = SemanticType::Scalar(ValueType::plain(registry.allocate_type_id()));
    let narrow = SemanticType::union([integer.clone(), string.clone()]);
    let wide = SemanticType::union([integer, string, decimal]);

    assert!(is_assignable(&narrow, &wide));
    assert!(!is_assignable(&wide, &narrow));
}

/// Verifies nullable lowering deduplicates repeated null members.
#[test]
fn nullable_unions_collapse_duplicate_members() {
    let mut registry = crate::Registry::new();
    let integer = SemanticType::Scalar(ValueType::plain(registry.allocate_type_id()));
    let null = SemanticType::Scalar(ValueType::plain(registry.allocate_type_id()));
    let nullable = SemanticType::union([integer.clone(), null.clone(), null]);

    assert!(is_assignable(&integer, &nullable));
    assert!(is_assignable(&nullable, &nullable));
    assert!(matches!(nullable, SemanticType::Union(members) if members.len() == 2));
}
