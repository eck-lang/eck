use super::*;

/// Verifies extraction of unsigned integer payloads and rejection of other representations.
#[test]
fn extracts_unsigned_integer_values_and_rejects_other_representations() {
    let unsigned_integer = Value::new(crate::integer::unsigned_integer8::test_type_id(), 42_u8);
    let float = Value::new(crate::integer::unsigned_integer8::test_type_id(), 42.0_f32);

    assert_eq!(get(&unsigned_integer).unwrap(), 42);
    assert!(matches!(
        get(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "uint8"
    ));
}
