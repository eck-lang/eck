use language_core::{CoreError, Value};

use crate::{operations::checked_addition, value::get as get_decimal};

/// Adds two decimal values and returns a decimal result.
pub(crate) fn addition_decimal(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(
        lhs.type_id(),
        checked_addition(get_decimal(lhs)?, get_decimal(rhs)?)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    /// Verifies decimal addition.
    #[test]
    fn adds_decimal_values() {
        let lhs = Value::new(crate::test_type_id(1), Decimal::new(15, 1));
        let rhs = Value::new(crate::test_type_id(1), Decimal::ONE);

        let result = addition_decimal(&lhs, &rhs).unwrap();

        assert_eq!(
            *result.downcast_ref::<Decimal>().unwrap(),
            Decimal::new(25, 1)
        );
    }
}
