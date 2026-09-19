//! Unit tests for typed expression semantic-shape helpers.

use super::*;

/// Verifies that array shape is read from the semantic output, not the node kind.
#[test]
fn array_shape_comes_from_semantic_output() {
    let mut registry = crate::semantic::Registry::new();
    let element = ValueType::plain(registry.allocate_type_id());
    let array_type = ArrayType {
        element,
        element_mode: crate::semantic::ArrayElementMode::Exact,
    };

    let scalar_expression = TypedExpression {
        kind: TypedExpressionKind::ArrayLiteral {
            elements: Vec::new(),
        },
        output: Some(SemanticType::Scalar(element)),
        span: crate::syntax::Span { start: 0, end: 0 },
    };
    assert_eq!(scalar_expression.array_type(), None);

    let array_expression = TypedExpression {
        output: Some(SemanticType::Array(array_type)),
        ..scalar_expression
    };
    assert_eq!(array_expression.array_type(), Some(array_type));
}

/// Verifies dense dispatch distinguishes both the base and subtype axes.
#[test]
fn complete_type_domain_selects_by_complete_value_type() {
    let mut registry = crate::semantic::Registry::new();
    let int64 = registry.allocate_type_id();
    let int128 = registry.allocate_type_id();
    let millimeter = registry.allocate_subtype_id();
    let candidates = [
        ValueType::plain(int64),
        ValueType::qualified(int64, millimeter),
        ValueType::plain(int128),
    ];
    let domain = CompleteTypeDomain::from_candidates(&registry, candidates);

    for (slot, candidate) in candidates.into_iter().enumerate() {
        assert_eq!(domain.candidate_index(&registry, candidate), Some(slot));
    }
    assert_eq!(
        domain.candidate_index(&registry, ValueType::qualified(int128, millimeter)),
        None
    );
}
