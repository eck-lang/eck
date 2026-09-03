use language_core::{CoreError, TypeId, Value};

/// Parses source text into a signed 32-bit integer value.
pub(crate) fn parse(raw_text: &str, type_id: TypeId) -> Result<Value, CoreError> {
    raw_text
        .parse::<i32>()
        .map(|value| Value::new(type_id, value))
        .map_err(|error| CoreError::InvalidLiteral {
            raw_text: raw_text.into(),
            type_name: "int32".into(),
            message: error.to_string(),
        })
}

#[cfg(test)]
#[path = "literal.tests.rs"]
mod tests;
