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

/// Promotes a finite single-precision floating-point value to decimal.
///
/// # Errors
///
/// Returns [`CoreError::Runtime`] when the floating-point value is not finite
/// or cannot be represented as a decimal.
pub(crate) fn from_float(value: f32) -> Result<Decimal, CoreError> {
    Decimal::try_from(value)
        .map_err(|error| CoreError::Runtime(format!("cannot convert float to decimal: {error}")))
}

/// Promotes a finite double-precision floating-point value to decimal.
///
/// # Errors
///
/// Returns [`CoreError::Runtime`] when the floating-point value is not finite
/// or cannot be represented as a decimal.
pub(crate) fn from_double(value: f64) -> Result<Decimal, CoreError> {
    Decimal::try_from(value)
        .map_err(|error| CoreError::Runtime(format!("cannot convert double to decimal: {error}")))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
