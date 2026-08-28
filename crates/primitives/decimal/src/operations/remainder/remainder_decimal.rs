use language_core::{CoreError, Value};

use crate::{operations::checked_remainder, value::get as get_decimal};

/// Calculates the remainder of two decimal values.
pub(crate) fn remainder_decimal(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let rhs = get_decimal(rhs)?;
    if rhs.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(
        lhs.type_id(),
        checked_remainder(get_decimal(lhs)?, rhs)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    /// Verifies decimal remainder and the zero-divisor error.
    #[test]
    fn calculates_decimal_remainder_and_rejects_zero() {
        let lhs = Value::new(crate::test_type_id(1), Decimal::new(105, 1));
        let rhs = Value::new(crate::test_type_id(1), Decimal::new(3, 0));
        let zero = Value::new(crate::test_type_id(1), Decimal::ZERO);

        let result = remainder_decimal(&lhs, &rhs).unwrap();

        assert_eq!(
            *result.downcast_ref::<Decimal>().unwrap(),
            Decimal::new(15, 1)
        );
        assert!(matches!(
            remainder_decimal(&lhs, &zero),
            Err(CoreError::DivisionByZero)
        ));
    }
}
