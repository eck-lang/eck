use super::*;

/// Verifies exact ordering, source order, signed boundaries, and invalid payloads.
#[test]
fn compares_promoted_integer16_operands() {
    let narrow = Value::new(crate::integer::integer8::test_type_id(), i8::MIN);
    let wide = Value::new(crate::integer::integer16::test_type_id(), i16::MAX);
    let invalid = Value::new(crate::integer::integer8::test_type_id(), false);

    assert!(less(&narrow, &wide).unwrap());
    assert!(greater(&wide, &narrow).unwrap());
    assert!(not_equal(&narrow, &wide).unwrap());
    assert!(matches!(
        equal(&invalid, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
