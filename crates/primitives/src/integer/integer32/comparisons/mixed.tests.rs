use super::*;

/// Verifies every narrower representation compares exactly in both source orders.
#[test]
fn compares_promoted_integer32_operands() {
    let wide = Value::new(crate::integer::integer32::test_type_id(), i32::MAX);
    for narrow in [
        Value::new(crate::integer::integer8::test_type_id(), i8::MIN),
        Value::new(crate::integer::integer16::test_type_id(), i16::MIN),
    ] {
        assert!(less(&narrow, &wide).unwrap());
        assert!(greater(&wide, &narrow).unwrap());
        assert!(not_equal(&narrow, &wide).unwrap());
    }

    let invalid = Value::new(crate::integer::integer8::test_type_id(), false);
    assert!(matches!(
        equal(&invalid, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
