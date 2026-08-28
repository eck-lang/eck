use language_core::{CoreError, Value};

use crate::operations::{checked_addition, decimal_int_operands};

/// Adds one decimal and one integer, regardless of operand order.
pub(crate) fn addition_decimal_int(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, decimal_id) = decimal_int_operands(lhs, rhs)?;
    Ok(Value::new(decimal_id, checked_addition(lhs, rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    use crate::value::get as get_decimal;

    /// Verifies decimal/integer addition in both operand orders.
    #[test]
    fn adds_decimal_and_integer_in_both_orders() {
        let decimal = Value::new(crate::test_type_id(1), Decimal::new(25, 1));
        let integer = Value::new(crate::test_type_id(2), 2_i64);

        let decimal_left = addition_decimal_int(&decimal, &integer).unwrap();
        let integer_left = addition_decimal_int(&integer, &decimal).unwrap();

        assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(45, 1));
        assert_eq!(get_decimal(&integer_left).unwrap(), Decimal::new(45, 1));
    }
}
