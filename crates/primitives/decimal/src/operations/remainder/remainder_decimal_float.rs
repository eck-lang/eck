use language_core::{CoreError, Value};

use crate::operations::{checked_remainder, decimal_float_operands};

/// Calculates the remainder of a decimal and a single-precision float in source order.
pub(crate) fn remainder_decimal_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, decimal_id) = decimal_float_operands(lhs, rhs)?;
    if rhs.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(decimal_id, checked_remainder(lhs, rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    use crate::value::get as get_decimal;

    #[test]
    fn calculates_decimal_float_remainder_in_both_orders() {
        assert_eq!(
            get_decimal(
                &remainder_decimal_float(
                    &Value::new(crate::test_type_id(1), Decimal::new(105, 1)),
                    &Value::new(crate::test_type_id(2), 2.0_f32),
                )
                .unwrap(),
            )
            .unwrap(),
            Decimal::new(5, 1)
        );
        assert_eq!(
            get_decimal(
                &remainder_decimal_float(
                    &Value::new(crate::test_type_id(2), 10.5_f32),
                    &Value::new(crate::test_type_id(1), Decimal::new(2, 0)),
                )
                .unwrap(),
            )
            .unwrap(),
            Decimal::new(5, 1)
        );
    }
}
