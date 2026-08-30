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
#[path = "formatting.tests.rs"]
mod tests;
