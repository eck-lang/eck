use language_core::{CoreError, Value};

use crate::operations::{checked_remainder, decimal_int_operands};

/// Calculates the remainder of a decimal and an integer in source order.
pub(crate) fn remainder_decimal_int(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, decimal_id) = decimal_int_operands(lhs, rhs)?;
    if rhs.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(decimal_id, checked_remainder(lhs, rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::get as get_decimal;
    use rust_decimal::Decimal;

    /// Verifies decimal/integer remainder in both operand orders.
    #[test]
    fn calculates_remainder_with_integer_in_both_orders() {
        let decimal_left = remainder_decimal_int(
            &Value::new(crate::test_type_id(1), Decimal::new(105, 1)),
            &Value::new(crate::test_type_id(2), 2_i64),
        )
        .unwrap();
        let integer_left = remainder_decimal_int(
            &Value::new(crate::test_type_id(2), 10_i64),
            &Value::new(crate::test_type_id(1), Decimal::new(3, 0)),
        )
        .unwrap();

        assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(5, 1));
        assert_eq!(get_decimal(&integer_left).unwrap(), Decimal::new(1, 0));
    }
}
