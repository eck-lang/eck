use super::*;
use num_bigint::BigInt;

/// Verifies parsing of ordinary values and of magnitudes beyond 128 bits.
#[test]
fn parses_arbitrary_precision_integer_literals() {
    let small = parse("42", crate::integer::bigint::test_type_id()).unwrap();
    let negative = parse("-42", crate::integer::bigint::test_type_id()).unwrap();
    let above_128_max = parse(
        "170141183460469231731687303715884105728",
        crate::integer::bigint::test_type_id(),
    )
    .unwrap();
    let below_128_min = parse(
        "-170141183460469231731687303715884105729",
        crate::integer::bigint::test_type_id(),
    )
    .unwrap();

    assert_eq!(*small.downcast_ref::<BigInt>().unwrap(), BigInt::from(42));
    assert_eq!(
        *negative.downcast_ref::<BigInt>().unwrap(),
        BigInt::from(-42)
    );
    assert_eq!(
        above_128_max.downcast_ref::<BigInt>().unwrap().to_string(),
        "170141183460469231731687303715884105728"
    );
    assert_eq!(
        below_128_min.downcast_ref::<BigInt>().unwrap().to_string(),
        "-170141183460469231731687303715884105729"
    );
}

/// Verifies that non-numeric source literals are rejected.
#[test]
fn rejects_non_numeric_literals() {
    for raw_text in ["not-an-integer", "12.5", ""] {
        assert!(matches!(
            parse(raw_text, crate::integer::bigint::test_type_id()),
            Err(CoreError::InvalidLiteral { .. })
        ));
    }
}
