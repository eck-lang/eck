use language_core::{CoreError, Value};
use rust_decimal::Decimal;

/// Extracts the decimal payload from a runtime value.
///
/// # Errors
///
/// Returns [`CoreError::InvalidValueRepresentation`] when `value` does not
/// contain a [`rust_decimal::Decimal`].
pub(crate) fn get(value: &Value) -> Result<Decimal, CoreError> {
    value
        .downcast_ref::<Decimal>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("decimal".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that a decimal payload can be extracted from a runtime value.
    #[test]
    fn extracts_decimal_value() {
        let value = Value::new(crate::test_type_id(7), Decimal::new(1250, 2));

        assert_eq!(get(&value).unwrap(), Decimal::new(1250, 2));
    }

    /// Verifies that values with another payload type are rejected.
    #[test]
    fn rejects_non_decimal_value() {
        let value = Value::new(crate::test_type_id(7), 1250_i64);

        assert!(
            matches!(get(&value), Err(CoreError::InvalidValueRepresentation(name)) if name == "decimal")
        );
    }
}
