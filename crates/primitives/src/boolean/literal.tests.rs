use super::*;

/// Verifies parsing of both canonical boolean literals.
#[test]
fn parses_true_and_false() {
    let type_id = crate::boolean::test_type_id();

    assert!(
        *parse("true", type_id)
            .unwrap()
            .downcast_ref::<bool>()
            .unwrap()
    );
    assert!(
        !*parse("false", type_id)
            .unwrap()
            .downcast_ref::<bool>()
            .unwrap()
    );
}
