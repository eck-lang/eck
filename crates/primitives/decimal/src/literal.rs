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
#[path = "literal.tests.rs"]
mod tests;
