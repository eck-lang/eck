use super::*;
use rust_decimal::Decimal;

/// Verifies decimal remainder and the zero-divisor error.
#[test]
fn calculates_decimal_remainder_and_rejects_zero() {
    let left_operand = Value::new(crate::decimal::test_type_id(1), Decimal::new(105, 1));
    let right_operand = Value::new(crate::decimal::test_type_id(1), Decimal::new(3, 0));
    let zero = Value::new(crate::decimal::test_type_id(1), Decimal::ZERO);

    let result = remainder_decimal(&left_operand, &right_operand).unwrap();

    assert_eq!(
        *result.downcast_ref::<Decimal>().unwrap(),
        Decimal::new(15, 1)
    );
    assert!(matches!(
        remainder_decimal(&left_operand, &zero),
        Err(CoreError::DivisionByZero)
    ));
}
