use super::*;

/// Verifies extraction of float payloads and rejection of other representations.
#[test]
fn extracts_float_values_and_rejects_other_representations() {
    let float = Value::new(crate::test_type_id(), 42.5_f32);
    let double = Value::new(crate::test_type_id(), 42.5_f64);

    assert_eq!(get(&float).unwrap(), 42.5);
    assert!(matches!(
        get(&double),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "float"
    ));
}
