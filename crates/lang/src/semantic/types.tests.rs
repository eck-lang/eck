//! Unit tests for the structural semantic types shared across compiler stages.

use super::*;
use crate::ArrayElementMode;

/// Verifies that an array semantic type retains its element representation mode.
#[test]
fn array_semantic_type_retains_element_mode() {
    let type_id = crate::Registry::new().allocate_type_id();
    let array_type = ArrayType {
        element: ValueType::plain(type_id),
        element_mode: ArrayElementMode::AdaptiveInt,
    };

    assert_eq!(
        SemanticType::Array(array_type),
        SemanticType::Array(array_type)
    );
    assert_eq!(array_type.element_mode, ArrayElementMode::AdaptiveInt);
}

/// Verifies that scalar and array shapes remain distinct semantic contracts.
#[test]
fn scalar_and_array_semantic_types_are_distinct() {
    let type_id = crate::Registry::new().allocate_type_id();
    let scalar_type = SemanticType::Scalar(ValueType::plain(type_id));
    let array_type = SemanticType::Array(ArrayType {
        element: ValueType::plain(type_id),
        element_mode: ArrayElementMode::Exact,
    });

    assert_ne!(scalar_type, array_type);
}
