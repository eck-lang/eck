use super::*;

/// Verifies canonical formatting and invalid runtime representation handling.
#[test]
fn formats_unsigned_integers_and_rejects_other_representations() {
    let unsigned_integer = Value::new(crate::integer::unsigned_integer64::test_type_id(), u64::MAX);
    let float = Value::new(crate::integer::unsigned_integer64::test_type_id(), 42.0_f32);

    assert_eq!(format(&unsigned_integer).unwrap(), "18446744073709551615");
    assert!(matches!(
        format(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "uint64"
    ));
}
