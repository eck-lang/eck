use super::*;

/// Verifies canonical float formatting and representation validation.
#[test]
fn formats_float_values_and_rejects_other_representations() {
    let float = Value::new(crate::float::test_type_id(), 16_777_216.0_f32);
    let double = Value::new(crate::float::test_type_id(), 1.0_f64);

    assert_eq!(format(&float).unwrap(), "16777216");
    assert!(matches!(
        format(&double),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "float"
    ));
}
