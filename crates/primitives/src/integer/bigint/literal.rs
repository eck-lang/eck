use language_core::{CoreError, TypeId, Value};
use num_bigint::BigInt;

/// Parses source text directly into an arbitrary-precision integer value.
///
/// Parsing targets `BigInt` itself so values larger than 128 bits work
/// without an intermediate fixed-width conversion. Only available memory
/// limits the accepted magnitude.
pub(crate) fn parse(raw_text: &str, type_id: TypeId) -> Result<Value, CoreError> {
    raw_text
        .parse::<BigInt>()
        .map(|value| Value::new(type_id, value))
        .map_err(|error| CoreError::InvalidLiteral {
            raw_text: raw_text.into(),
            type_name: "bigint".into(),
            message: error.to_string(),
        })
}

#[cfg(test)]
#[path = "literal.tests.rs"]
mod tests;
