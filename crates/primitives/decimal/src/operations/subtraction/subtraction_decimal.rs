use language_core::{CoreError, Value};

use crate::{operations::checked_subtraction, value::get as get_decimal};

/// Subtracts the right decimal from the left decimal.
pub(crate) fn subtraction_decimal(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(
        lhs.type_id(),
        checked_subtraction(get_decimal(lhs)?, get_decimal(rhs)?)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    /// Verifies decimal subtraction.
    #[test]
    fn subtracts_decimal_values() {
        let lhs = Value::new(crate::test_type_id(1), Decimal::new(25, 1));
        let rhs = Value::new(crate::test_type_id(1), Decimal::ONE);

        let result = subtraction_decimal(&lhs, &rhs).unwrap();

        assert_eq!(
            *result.downcast_ref::<Decimal>().unwrap(),
            Decimal::new(15, 1)
        );
    }
}
