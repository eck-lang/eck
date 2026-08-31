use language_core::{CoreError, TypeId, Value};

/// Parses source text into an unsigned 8-bit integer value.
pub(crate) fn parse(raw_text: &str, type_id: TypeId) -> Result<Value, CoreError> {
    raw_text
        .parse::<u8>()
        .map(|value| Value::new(type_id, value))
        .map_err(|error| CoreError::InvalidLiteral {
            raw_text: raw_text.into(),
            type_name: "uint8".into(),
            message: error.to_string(),
        })
}

#[cfg(test)]
#[path = "literal.tests.rs"]
mod tests;
