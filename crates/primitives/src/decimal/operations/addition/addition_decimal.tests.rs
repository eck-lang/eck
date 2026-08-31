use super::*;
use rust_decimal::Decimal;

/// Verifies decimal addition.
#[test]
fn adds_decimal_values() {
    let left_operand = Value::new(crate::decimal::test_type_id(1), Decimal::new(15, 1));
    let right_operand = Value::new(crate::decimal::test_type_id(1), Decimal::ONE);

    let result = addition_decimal(&left_operand, &right_operand).unwrap();

    assert_eq!(
        *result.downcast_ref::<Decimal>().unwrap(),
        Decimal::new(25, 1)
    );
}
