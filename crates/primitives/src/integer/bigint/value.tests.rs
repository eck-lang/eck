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

/// Verifies mixed operands widen every narrower width and reject invalid pairs.
#[test]
fn widens_narrower_operands_to_bigint_in_source_order() {
    let wider_id = crate::integer::bigint::test_type_id();
    let wide = Value::new(wider_id, BigInt::from(100));
    let narrow8 = Value::new(crate::integer::integer8::test_type_id(), 7_i8);
    let narrow16 = Value::new(crate::integer::integer16::test_type_id(), 7_i16);
    let narrow32 = Value::new(crate::integer::integer32::test_type_id(), 7_i32);
    let narrow64 = Value::new(crate::integer::integer64::test_type_id(), 7_i64);
    let narrow128 = Value::new(crate::integer::integer128::test_type_id(), 7_i128);

    for narrow in [&narrow8, &narrow16, &narrow32, &narrow64, &narrow128] {
        let (left_operand, right_operand, result_type_id) =
            mixed_operands(&wide, narrow).unwrap();
        assert_eq!((left_operand, right_operand), (BigInt::from(100), BigInt::from(7)));
        assert_eq!(result_type_id, wider_id);
        let (left_operand, right_operand, result_type_id) =
            mixed_operands(narrow, &wide).unwrap();
        assert_eq!((left_operand, right_operand), (BigInt::from(7), BigInt::from(100)));
        assert_eq!(result_type_id, wider_id);
    }
    let float = Value::new(wider_id, 7.0_f32);
    assert!(matches!(
        mixed_operands(&float, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
    assert!(matches!(
        mixed_operands(&wide, &wide),
        Err(CoreError::InvalidValueRepresentation(_))
    ));
}
