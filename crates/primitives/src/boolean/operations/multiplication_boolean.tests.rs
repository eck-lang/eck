use language_core::Value;
use rust_decimal::Decimal;

use super::{multiplication_boolean_numeric, multiplication_numeric_boolean};

/// Allocates a boolean-crate test type identifier.
fn type_id() -> language_core::TypeId {
    crate::boolean::test_type_id()
}

/// Verifies that multiplying by true preserves the original numeric value.
#[test]
fn true_returns_the_numeric_operand_unchanged() {
    let numeric = Value::new(type_id(), Decimal::new(125, 2));
    let boolean = Value::new(type_id(), true);

    let result = multiplication_numeric_boolean(&numeric, &boolean).unwrap();

    assert_eq!(
        *result.downcast_ref::<Decimal>().unwrap(),
        Decimal::new(125, 2)
    );
}

/// Verifies zero multiplication for every supported numeric representation.
#[test]
fn false_multiplies_every_supported_numeric_representation_by_zero() {
    let boolean = Value::new(type_id(), false);

    let integer = multiplication_numeric_boolean(&Value::new(type_id(), 7_i64), &boolean).unwrap();
    let float = multiplication_numeric_boolean(&Value::new(type_id(), 1.5_f32), &boolean).unwrap();
    let double = multiplication_numeric_boolean(&Value::new(type_id(), 1.5_f64), &boolean).unwrap();
    let decimal =
        multiplication_numeric_boolean(&Value::new(type_id(), Decimal::new(15, 1)), &boolean)
            .unwrap();

    assert_eq!(*integer.downcast_ref::<i64>().unwrap(), 0);
    assert_eq!(*float.downcast_ref::<f32>().unwrap(), 0.0);
    assert_eq!(*double.downcast_ref::<f64>().unwrap(), 0.0);
    assert_eq!(*decimal.downcast_ref::<Decimal>().unwrap(), Decimal::ZERO);
}

/// Verifies multiplication when the boolean is the left operand.
#[test]
fn supports_a_boolean_as_the_left_operand() {
    let boolean = Value::new(type_id(), false);
    let numeric = Value::new(type_id(), 7_i64);

    let result = multiplication_boolean_numeric(&boolean, &numeric).unwrap();

    assert_eq!(*result.downcast_ref::<i64>().unwrap(), 0);
}
