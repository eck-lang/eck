use super::*;

/// Verifies every narrower representation compares exactly in both source orders.
#[test]
fn compares_promoted_integer64_operands() {
    let wide = Value::new(
        crate::primitives::integer::integer64::test_type_id(),
        i64::MAX,
    );
    for narrow in [
        Value::new(
            crate::primitives::integer::integer8::test_type_id(),
            i8::MIN,
        ),
        Value::new(
            crate::primitives::integer::integer16::test_type_id(),
            i16::MIN,
        ),
        Value::new(
            crate::primitives::integer::integer32::test_type_id(),
            i32::MIN,
        ),
    ] {
        assert!(less(&narrow, &wide).unwrap());
        assert!(greater(&wide, &narrow).unwrap());
        assert!(not_equal(&narrow, &wide).unwrap());
    }

    let invalid = Value::new(crate::primitives::integer::integer8::test_type_id(), false);
    assert!(matches!(
        equal(&invalid, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
