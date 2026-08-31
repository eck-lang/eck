use super::*;

/// Verifies extraction of double payloads and rejection of other representations.
#[test]
fn extracts_double_values_and_rejects_other_representations() {
    let double = Value::new(crate::double::test_type_id(), 42.5_f64);
    let float = Value::new(crate::double::test_type_id(), 42.5_f32);

    assert_eq!(get(&double).unwrap(), 42.5);
    assert!(matches!(
        get(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "double"
    ));
}
