use super::*;
use num_bigint::BigInt;

/// Verifies extraction of integer payloads and rejection of other representations.
#[test]
fn extracts_integer_values_and_rejects_other_representations() {
    let integer = Value::new(crate::integer::bigint::test_type_id(), BigInt::from(42));
    let float = Value::new(crate::integer::bigint::test_type_id(), 42.0_f32);

    assert_eq!(get(&integer).unwrap(), BigInt::from(42));
    assert!(matches!(
        get(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "bigint"
    ));
}
