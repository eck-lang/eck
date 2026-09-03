use super::*;
use num_bigint::BigInt;

/// Verifies canonical formatting and invalid runtime representation handling.
#[test]
fn formats_integers_and_rejects_other_representations() {
    let integer = Value::new(
        crate::integer::bigint::test_type_id(),
        "-170141183460469231731687303715884105729"
            .parse::<BigInt>()
            .unwrap(),
    );
    let float = Value::new(crate::integer::bigint::test_type_id(), 42.0_f32);

    assert_eq!(
        format(&integer).unwrap(),
        "-170141183460469231731687303715884105729"
    );
    assert!(matches!(
        format(&float),
        Err(CoreError::InvalidValueRepresentation(name)) if name == "bigint"
    ));
}
