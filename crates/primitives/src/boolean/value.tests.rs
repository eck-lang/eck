use super::*;

/// Verifies extraction of boolean payloads and rejection of other representations.
#[test]
fn extracts_boolean_values_and_rejects_other_representations() {
    let boolean = Value::new(crate::boolean::test_type_id(), true);
    let integer = Value::new(crate::boolean::test_type_id(), 1_i64);

    assert!(get(&boolean).unwrap());
    assert!(matches!(
        get(&integer),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "bool"
    ));
}
