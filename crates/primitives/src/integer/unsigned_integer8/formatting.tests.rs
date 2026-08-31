use super::*;

/// Verifies canonical formatting and invalid runtime representation handling.
#[test]
fn formats_unsigned_integers_and_rejects_other_representations() {
    let unsigned_integer = Value::new(crate::integer::unsigned_integer8::test_type_id(), u8::MAX);
    let float = Value::new(crate::integer::unsigned_integer8::test_type_id(), 42.0_f32);

    assert_eq!(format(&unsigned_integer).unwrap(), "255");
    assert!(matches!(
        format(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "uint8"
    ));
}
