use super::*;
use crate::ir::TypedStatement;

/// Plain adaptive flow facts prune subtype alternatives without pruning widening widths.
#[test]
fn proven_plain_adaptive_elements_keep_all_signed_widths() {
    let registry = crate::default_registry().unwrap();
    let program = crate::compile(
        &crate::parse("let values: int[] = [1,2]\nlet index = 0\nlet value = values[index]")
            .unwrap(),
        &registry,
    )
    .unwrap();
    let TypedStatement::VariableDeclaration { expression, .. } = &program.statements()[2] else {
        panic!();
    };
    let TypedExpressionKind::ElementAccess {
        type_domain: Some(domain),
        ..
    } = &expression.kind
    else {
        panic!("adaptive reads retain finite width dispatch");
    };
    assert!(
        domain
            .candidates
            .iter()
            .all(|candidate| candidate.subtype.is_none())
    );
    for base in registry.signed_integer_widening_types() {
        assert!(domain.candidates.contains(&ValueType::plain(base)));
    }
}

/// A qualified observation keeps the full conservative subtype contract for typed reads.
#[test]
fn qualified_adaptive_elements_retain_subtype_dispatch() {
    let registry = crate::default_registry().unwrap();
    let program = crate::compile(
        &crate::parse("let values: int[] = [10mm]\nlet index = 0\nlet value = values[index]")
            .unwrap(),
        &registry,
    )
    .unwrap();
    let TypedStatement::VariableDeclaration { expression, .. } = &program.statements()[2] else {
        panic!();
    };
    let TypedExpressionKind::ElementAccess {
        type_domain: Some(domain),
        ..
    } = &expression.kind
    else {
        panic!();
    };
    assert!(
        domain
            .candidates
            .iter()
            .any(|candidate| candidate.subtype.is_some())
    );
    assert!(
        domain
            .candidates
            .iter()
            .any(|candidate| candidate.subtype.is_none())
    );
}
