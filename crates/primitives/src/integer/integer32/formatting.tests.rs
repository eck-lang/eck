use super::*;

/// Verifies canonical formatting and invalid runtime representation handling.
#[test]
fn formats_integers_and_rejects_other_representations() {
    let integer = Value::new(crate::integer::integer32::test_type_id(), i32::MIN);
    let float = Value::new(crate::integer::integer32::test_type_id(), 42.0_f32);

    assert_eq!(format(&integer).unwrap(), "-2147483648");
    assert!(matches!(
        format(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "int32"
    ));
}
