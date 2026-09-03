use super::*;

/// Verifies extraction of integer payloads and rejection of other representations.
#[test]
fn extracts_integer_values_and_rejects_other_representations() {
    let integer = Value::new(crate::integer::integer16::test_type_id(), 42_i16);
    let float = Value::new(crate::integer::integer16::test_type_id(), 42.0_f32);

    assert_eq!(get(&integer).unwrap(), 42);
    assert!(matches!(
        get(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "int16"
    ));
}

/// Verifies `int8` operands widen exactly to `int16` in either source order.
#[test]
fn widens_mixed_integer16_operands() {
    let wider_id = crate::integer::integer16::test_type_id();
    let narrow = Value::new(crate::integer::integer8::test_type_id(), i8::MIN);
    let wide = Value::new(wider_id, i16::MAX);

    assert_eq!(
        mixed_operands(&narrow, &wide).unwrap(),
        (i16::from(i8::MIN), i16::MAX, wider_id)
    );
    assert_eq!(
        mixed_operands(&wide, &narrow).unwrap(),
        (i16::MAX, i16::from(i8::MIN), wider_id)
    );
    let invalid = Value::new(crate::integer::integer8::test_type_id(), false);
    assert!(matches!(
        mixed_operands(&invalid, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
