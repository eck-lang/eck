use std::str::FromStr;

use language_core::{CoreError, TypeId, Value};
use rust_decimal::Decimal;

/// Parses a decimal literal from its original source representation.
///
/// The literal is parsed directly into [`rust_decimal::Decimal`] without an
/// intermediate floating-point conversion.
///
/// # Errors
///
/// Returns [`CoreError::InvalidLiteral`] when `raw_text` is not a valid decimal.
pub(crate) fn parse(raw_text: &str, type_id: TypeId) -> Result<Value, CoreError> {
    Decimal::from_str(raw_text)
        .map(|value| Value::new(type_id, value))
        .map_err(|error| CoreError::InvalidLiteral {
            raw_text: raw_text.into(),
            type_name: "decimal".into(),
            message: error.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that decimal literals preserve their exact source precision.
    #[test]
    fn parses_decimal_literal_without_float_conversion() {
        let value = parse("12.500", crate::test_type_id(7)).unwrap();

        assert_eq!(value.type_id(), crate::test_type_id(7));
        assert_eq!(
            *value.downcast_ref::<Decimal>().unwrap(),
            Decimal::new(12500, 3)
        );
    }

    /// Verifies that invalid decimal literals return a typed compiler error.
    #[test]
    fn rejects_invalid_decimal_literal() {
        let error = parse("not-a-decimal", crate::test_type_id(7))
            .err()
            .unwrap();

        assert!(
            matches!(error, CoreError::InvalidLiteral { raw_text, type_name, .. }
            if raw_text == "not-a-decimal" && type_name == "decimal")
        );
    }
}
