use language_core::{CoreError, Value};

use crate::operations::{checked_subtraction, decimal_float_operands};

/// Subtracts a decimal and a single-precision float while preserving source order.
pub(crate) fn subtraction_decimal_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, decimal_id) = decimal_float_operands(lhs, rhs)?;
    Ok(Value::new(decimal_id, checked_subtraction(lhs, rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    use crate::value::get as get_decimal;

    #[test]
    fn subtracts_decimal_and_float_in_both_orders() {
        let decimal = Value::new(crate::test_type_id(1), Decimal::new(25, 1));
        let float = Value::new(crate::test_type_id(2), 2.0_f32);

        assert_eq!(
            get_decimal(&subtraction_decimal_float(&decimal, &float).unwrap()).unwrap(),
            Decimal::new(5, 1)
        );
        assert_eq!(
            get_decimal(&subtraction_decimal_float(&float, &decimal).unwrap()).unwrap(),
            Decimal::new(-5, 1)
        );
    }
}
