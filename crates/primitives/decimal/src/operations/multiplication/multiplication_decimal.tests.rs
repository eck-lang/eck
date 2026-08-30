use super::*;
use rust_decimal::Decimal;

/// Verifies decimal multiplication.
#[test]
fn multiplies_decimal_values() {
    let left_operand = Value::new(crate::test_type_id(1), Decimal::new(25, 1));
    let right_operand = Value::new(crate::test_type_id(1), Decimal::new(2, 0));

    let result = multiplication_decimal(&left_operand, &right_operand).unwrap();

    assert_eq!(
        *result.downcast_ref::<Decimal>().unwrap(),
        Decimal::new(5, 0)
    );
}
