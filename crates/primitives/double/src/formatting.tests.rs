use super::*;

/// Verifies canonical double formatting and representation validation.
#[test]
fn formats_double_values_and_rejects_other_representations() {
    let double = Value::new(crate::test_type_id(), 16_777_217.0_f64);
    let float = Value::new(crate::test_type_id(), 1.0_f32);

    assert_eq!(format(&double).unwrap(), "16777217");
    assert!(matches!(
        format(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "double"
    ));
}
