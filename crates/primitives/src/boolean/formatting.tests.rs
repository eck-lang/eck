use super::*;

/// Verifies canonical formatting and invalid runtime representation handling.
#[test]
fn formats_booleans_and_rejects_other_representations() {
    let boolean = Value::new(crate::boolean::test_type_id(), false);
    let integer = Value::new(crate::boolean::test_type_id(), 0_i64);

    assert_eq!(format(&boolean).unwrap(), "false");
    assert!(matches!(
        format(&integer),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "bool"
    ));
}
