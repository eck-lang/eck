use language_core::{CoreError, Value};

use crate::operations::{checked_division, decimal_int_operands};

/// Divides a decimal and an integer while preserving their source order.
pub(crate) fn division_decimal_int(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, decimal_id) = decimal_int_operands(lhs, rhs)?;
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

    /// Verifies decimal/integer division in both operand orders.
    #[test]
    fn divides_decimal_and_integer_in_both_orders() {
        let decimal_left = division_decimal_int(
            &Value::new(crate::test_type_id(1), Decimal::new(5, 0)),
            &Value::new(crate::test_type_id(2), 2_i64),
        )
        .unwrap();
        let integer_left = division_decimal_int(
            &Value::new(crate::test_type_id(2), 5_i64),
            &Value::new(crate::test_type_id(1), Decimal::new(2, 0)),
        )
        .unwrap();

        assert_eq!(get_decimal(&decimal_left).unwrap(), Decimal::new(25, 1));
        assert_eq!(get_decimal(&integer_left).unwrap(), Decimal::new(25, 1));
    }
}
