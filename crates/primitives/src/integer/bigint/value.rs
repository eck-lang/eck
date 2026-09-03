use language_core::{CoreError, Value};
use num_bigint::BigInt;

/// Extracts the arbitrary-precision integer payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<BigInt, CoreError> {
    value
        .downcast_ref::<BigInt>()
        .cloned()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("bigint".into()))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
