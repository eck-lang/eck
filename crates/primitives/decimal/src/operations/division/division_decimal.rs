use language_core::{CoreError, Value};

use crate::{operations::checked_division, value::get as get_decimal};

/// Divides the left decimal by the right decimal.
pub(crate) fn division_decimal(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let rhs = get_decimal(rhs)?;
    if rhs.is_zero() {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(
        lhs.type_id(),
        checked_division(get_decimal(lhs)?, rhs)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    /// Verifies decimal division and the zero-divisor error.
    #[test]
    fn divides_decimal_values_and_rejects_zero() {
        let lhs = Value::new(crate::test_type_id(1), Decimal::new(5, 0));
        let rhs = Value::new(crate::test_type_id(1), Decimal::new(2, 0));
        let zero = Value::new(crate::test_type_id(1), Decimal::ZERO);

        let result = division_decimal(&lhs, &rhs).unwrap();

        assert_eq!(
            *result.downcast_ref::<Decimal>().unwrap(),
            Decimal::new(25, 1)
        );
        assert!(matches!(
            division_decimal(&lhs, &zero),
            Err(CoreError::DivisionByZero)
        ));
    }
}
