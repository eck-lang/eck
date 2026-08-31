use language_core::{CoreError, TypeId, Value};

/// Parses source text into a signed 64-bit integer value.
pub(crate) fn parse(raw_text: &str, type_id: TypeId) -> Result<Value, CoreError> {
    raw_text
        .parse::<i64>()
        .map(|value| Value::new(type_id, value))
        .map_err(|error| CoreError::InvalidLiteral {
            raw_text: raw_text.into(),
            type_name: "int64".into(),
            message: error.to_string(),
        })
}

#[cfg(test)]
#[path = "literal.tests.rs"]
mod tests;
