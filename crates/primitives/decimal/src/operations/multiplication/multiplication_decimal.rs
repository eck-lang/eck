use language_core::{CoreError, Value};

use crate::{operations::checked_multiplication, value::get as get_decimal};

/// Multiplies two decimal values and returns a decimal result.
pub(crate) fn multiplication_decimal(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(
        lhs.type_id(),
        checked_multiplication(get_decimal(lhs)?, get_decimal(rhs)?)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    /// Verifies decimal multiplication.
    #[test]
    fn multiplies_decimal_values() {
        let lhs = Value::new(crate::test_type_id(1), Decimal::new(25, 1));
        let rhs = Value::new(crate::test_type_id(1), Decimal::new(2, 0));

        let result = multiplication_decimal(&lhs, &rhs).unwrap();

        assert_eq!(
            *result.downcast_ref::<Decimal>().unwrap(),
            Decimal::new(5, 0)
        );
    }
}
