use language_core::{CoreError, Value};

use crate::operations::{checked_subtraction, decimal_double_operands};

/// Subtracts a decimal and a double while preserving their source order.
pub(crate) fn subtraction_decimal_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, decimal_id) = decimal_double_operands(lhs, rhs)?;
    Ok(Value::new(decimal_id, checked_subtraction(lhs, rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    use crate::value::get as get_decimal;

    /// Verifies decimal/double subtraction in both operand orders.
    #[test]
    fn subtracts_decimal_and_double_in_both_orders() {
        let decimal = Value::new(crate::test_type_id(1), Decimal::new(25, 1));
        let double = Value::new(crate::test_type_id(2), 2.0_f64);

        let decimal_left = subtraction_decimal_double(&decimal, &double).unwrap();
        let double_left = subtraction_decimal_double(&double, &decimal).unwrap();

        assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(5, 1));
        assert_eq!(get_decimal(&double_left).unwrap(), Decimal::new(-5, 1));
    }
}
