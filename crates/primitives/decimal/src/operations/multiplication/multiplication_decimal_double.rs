use language_core::{CoreError, Value};

use crate::operations::{checked_multiplication, decimal_double_operands};

/// Multiplies one decimal and one double, regardless of operand order.
pub(crate) fn multiplication_decimal_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, decimal_id) = decimal_double_operands(lhs, rhs)?;
    Ok(Value::new(decimal_id, checked_multiplication(lhs, rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::get as get_decimal;
    use rust_decimal::Decimal;

    /// Verifies decimal/double multiplication in both operand orders.
    #[test]
    fn multiplies_decimal_and_double_in_both_orders() {
        let decimal = Value::new(crate::test_type_id(1), Decimal::new(25, 1));
        let double = Value::new(crate::test_type_id(2), 2.0_f64);

        let decimal_left = multiplication_decimal_double(&decimal, &double).unwrap();
        let double_left = multiplication_decimal_double(&double, &decimal).unwrap();

        assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(5, 0));
        assert_eq!(get_decimal(&double_left).unwrap(), Decimal::new(5, 0));
    }
}
