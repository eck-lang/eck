use language_core::{CoreError, Value};

use crate::operations::{checked_division, decimal_double_operands};

/// Divides a decimal and a double while preserving their source order.
pub(crate) fn division_decimal_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, decimal_id) = decimal_double_operands(lhs, rhs)?;
    if rhs.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(decimal_id, checked_division(lhs, rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::get as get_decimal;
    use rust_decimal::Decimal;

    /// Verifies decimal/double division in both operand orders.
    #[test]
    fn divides_decimal_and_double_in_both_orders() {
        let decimal = Value::new(crate::test_type_id(1), Decimal::new(5, 0));
        let double = Value::new(crate::test_type_id(2), 2.0_f64);

        let decimal_left = division_decimal_double(&decimal, &double).unwrap();
        let double_left = division_decimal_double(
            &Value::new(crate::test_type_id(2), 5.0_f64),
            &Value::new(crate::test_type_id(1), Decimal::new(2, 0)),
        )
        .unwrap();

        assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(25, 1));
        assert_eq!(get_decimal(&double_left).unwrap(), Decimal::new(25, 1));
    }
}
