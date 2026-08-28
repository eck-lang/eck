use language_core::{CoreError, Value};

use crate::{
    operations::{checked_power, decimal_exponent},
    value::get as get_decimal,
};

/// Raises a decimal to an integer-valued decimal exponent.
pub(crate) fn power_decimal(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let exponent = decimal_exponent(get_decimal(rhs)?)?;
    Ok(Value::new(
        lhs.type_id(),
        checked_power(get_decimal(lhs)?, exponent)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    /// Verifies positive, negative, overflowing and invalid decimal powers.
    #[test]
    fn calculates_decimal_power_and_rejects_fractional_exponents() {
        let positive = power_decimal(
            &Value::new(crate::test_type_id(1), Decimal::new(25, 1)),
            &Value::new(crate::test_type_id(1), Decimal::new(3, 0)),
        )
        .unwrap();
        let negative = power_decimal(
            &Value::new(crate::test_type_id(1), Decimal::new(2, 0)),
            &Value::new(crate::test_type_id(1), Decimal::new(-2, 0)),
        )
        .unwrap();
        let fractional = power_decimal(
            &Value::new(crate::test_type_id(1), Decimal::new(2, 0)),
            &Value::new(crate::test_type_id(1), Decimal::new(15, 1)),
        );
        let zero_to_negative = power_decimal(
            &Value::new(crate::test_type_id(1), Decimal::ZERO),
            &Value::new(crate::test_type_id(1), Decimal::NEGATIVE_ONE),
        );
        let overflow = power_decimal(
            &Value::new(crate::test_type_id(1), Decimal::MAX),
            &Value::new(crate::test_type_id(1), Decimal::new(2, 0)),
        );

        assert_eq!(
            *positive.downcast_ref::<Decimal>().unwrap(),
            Decimal::new(15625, 3)
        );
        assert_eq!(
            *negative.downcast_ref::<Decimal>().unwrap(),
            Decimal::new(25, 2)
        );
        assert!(
            matches!(fractional, Err(CoreError::Runtime(message)) if message.contains("integer"))
        );
        assert!(matches!(zero_to_negative, Err(CoreError::DivisionByZero)));
        assert!(
            matches!(overflow, Err(CoreError::Runtime(message)) if message.contains("overflow"))
        );
    }
}
