use super::*;

/// Verifies parsing of the canonical null literal.
#[test]
fn parses_null() {
    let type_id = crate::primitives::null::test_type_id();

    assert!(
        parse("null", type_id)
            .unwrap()
            .downcast_ref::<crate::primitives::null::value::Null>()
            .is_some()
    );
}

/// Verifies that non-canonical text is rejected as an invalid null literal.
#[test]
fn rejects_non_null_text() {
    let type_id = crate::primitives::null::test_type_id();

    assert!(matches!(
        parse("NULL", type_id),
        Err(crate::semantic::CoreError::InvalidLiteral { .. })
    ));
    assert!(matches!(
        parse("", type_id),
        Err(crate::semantic::CoreError::InvalidLiteral { .. })
    ));
}
