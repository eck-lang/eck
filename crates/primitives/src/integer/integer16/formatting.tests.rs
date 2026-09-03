use super::*;

/// Verifies canonical formatting and invalid runtime representation handling.
#[test]
fn formats_integers_and_rejects_other_representations() {
    let integer = Value::new(crate::integer::integer16::test_type_id(), i16::MIN);
    let float = Value::new(crate::integer::integer16::test_type_id(), 42.0_f32);

    assert_eq!(format(&integer).unwrap(), "-32768");
    assert!(matches!(
        format(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "int16"
    ));
}
