use language_core::{CoreError, Value};

use crate::value::get;

/// Formats a runtime decimal value as a string.
///
/// The decimal is rendered using [`rust_decimal::Decimal::to_string`], which
/// preserves the decimal representation without converting through a binary
/// floating-point type.
///
/// # Errors
///
/// Returns [`CoreError::InvalidValueRepresentation`] if `value` does not
/// contain a [`rust_decimal::Decimal`].
pub(crate) fn format(value: &Value) -> Result<String, CoreError> {
    Ok(get(value)?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    /// Verifies that formatting preserves a decimal's stored scale.
    #[test]
    fn formats_decimal_without_binary_float_conversion() {
        let value = Value::new(crate::test_type_id(7), Decimal::new(12500, 3));

        assert_eq!(format(&value).unwrap(), "12.500");
    }

    /// Verifies that formatting rejects values with a different payload type.
    #[test]
    fn rejects_non_decimal_value() {
        let value = Value::new(crate::test_type_id(7), 1250_i64);

        assert!(matches!(
            format(&value),
            Err(CoreError::InvalidValueRepresentation(name)) if name == "decimal"
        ));
    }
}
