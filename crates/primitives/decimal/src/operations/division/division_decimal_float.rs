use language_core::{CoreError, Value};

use crate::operations::{checked_division, decimal_float_operands};

/// Divides a decimal and a single-precision float while preserving source order.
pub(crate) fn division_decimal_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, decimal_id) = decimal_float_operands(lhs, rhs)?;
    if rhs.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(decimal_id, checked_division(lhs, rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    use crate::value::get as get_decimal;

    #[test]
    fn divides_decimal_and_float_in_both_orders() {
        let decimal = Value::new(crate::test_type_id(1), Decimal::new(5, 0));
        let float = Value::new(crate::test_type_id(2), 2.0_f32);

        assert_eq!(
            get_decimal(&division_decimal_float(&decimal, &float).unwrap()).unwrap(),
            Decimal::new(25, 1)
        );
        assert_eq!(
            get_decimal(
                &division_decimal_float(
                    &Value::new(crate::test_type_id(2), 5.0_f32),
                    &Value::new(crate::test_type_id(1), Decimal::new(2, 0)),
                )
                .unwrap(),
            )
            .unwrap(),
            Decimal::new(25, 1)
        );
    }
}
