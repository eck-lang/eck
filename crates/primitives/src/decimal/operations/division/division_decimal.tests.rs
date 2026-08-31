use super::*;
use rust_decimal::Decimal;

/// Verifies decimal division and the zero-divisor error.
#[test]
fn divides_decimal_values_and_rejects_zero() {
    let left_operand = Value::new(crate::decimal::test_type_id(1), Decimal::new(5, 0));
    let right_operand = Value::new(crate::decimal::test_type_id(1), Decimal::new(2, 0));
    let zero = Value::new(crate::decimal::test_type_id(1), Decimal::ZERO);

    let result = division_decimal(&left_operand, &right_operand).unwrap();

    assert_eq!(
        *result.downcast_ref::<Decimal>().unwrap(),
        Decimal::new(25, 1)
    );
    assert!(matches!(
        division_decimal(&left_operand, &zero),
        Err(CoreError::DivisionByZero)
    ));
}
