use super::*;

/// Verifies extraction of integer payloads and rejection of other representations.
#[test]
fn extracts_integer_values_and_rejects_other_representations() {
    let integer = Value::new(crate::integer::integer32::test_type_id(), 42_i32);
    let float = Value::new(crate::integer::integer32::test_type_id(), 42.0_f32);

    assert_eq!(get(&integer).unwrap(), 42);
    assert!(matches!(
        get(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "int32"
    ));
}

/// Verifies every narrower signed representation widens exactly to `int32`.
#[test]
fn widens_mixed_integer32_operands() {
    let wider_id = crate::integer::integer32::test_type_id();
    let wide = Value::new(wider_id, i32::MAX);
    for (narrow, expected) in [
        (
            Value::new(crate::integer::integer8::test_type_id(), i8::MIN),
            i32::from(i8::MIN),
        ),
        (
            Value::new(crate::integer::integer16::test_type_id(), i16::MIN),
            i32::from(i16::MIN),
        ),
    ] {
        assert_eq!(
            mixed_operands(&narrow, &wide).unwrap(),
            (expected, i32::MAX, wider_id)
        );
        assert_eq!(
            mixed_operands(&wide, &narrow).unwrap(),
            (i32::MAX, expected, wider_id)
        );
    }
    let invalid = Value::new(crate::integer::integer8::test_type_id(), false);
    assert!(matches!(
        mixed_operands(&invalid, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
