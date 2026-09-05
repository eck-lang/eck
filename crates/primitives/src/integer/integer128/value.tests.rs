use super::*;

/// Verifies extraction of integer payloads and rejection of other representations.
#[test]
fn extracts_integer_values_and_rejects_other_representations() {
    let integer = Value::new(crate::integer::integer128::test_type_id(), 42_i128);
    let float = Value::new(crate::integer::integer128::test_type_id(), 42.0_f32);

    assert_eq!(get(&integer).unwrap(), 42);
    assert!(matches!(
        get(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "int128"
    ));
}

/// Verifies every narrower signed representation widens exactly to `int128`.
#[test]
fn widens_mixed_integer128_operands() {
    let wider_id = crate::integer::integer128::test_type_id();
    let wide = Value::new(wider_id, i128::MAX);
    for (narrow, expected) in [
        (
            Value::new(crate::integer::integer8::test_type_id(), i8::MIN),
            i128::from(i8::MIN),
        ),
        (
            Value::new(crate::integer::integer16::test_type_id(), i16::MIN),
            i128::from(i16::MIN),
        ),
        (
            Value::new(crate::integer::integer32::test_type_id(), i32::MIN),
            i128::from(i32::MIN),
        ),
        (
            Value::new(crate::integer::integer64::test_type_id(), i64::MIN),
            i128::from(i64::MIN),
        ),
    ] {
        assert_eq!(
            mixed_operands(&narrow, &wide).unwrap(),
            (expected, i128::MAX, wider_id)
        );
        assert_eq!(
            mixed_operands(&wide, &narrow).unwrap(),
            (i128::MAX, expected, wider_id)
        );
    }
    let invalid = Value::new(crate::integer::integer8::test_type_id(), false);
    assert!(matches!(
        mixed_operands(&invalid, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
