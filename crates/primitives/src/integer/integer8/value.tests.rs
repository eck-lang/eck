use super::*;

/// Verifies extraction of integer payloads and rejection of other representations.
#[test]
fn extracts_integer_values_and_rejects_other_representations() {
    let integer = Value::new(crate::integer::integer8::test_type_id(), 42_i8);
    let float = Value::new(crate::integer::integer8::test_type_id(), 42.0_f32);

    assert_eq!(get(&integer).unwrap(), 42);
    assert!(matches!(
        get(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "int8"
    ));
}
