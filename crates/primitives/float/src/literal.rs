use language_core::{CoreError, TypeId, Value};

/// Parses source text into a single-precision floating-point value.
pub(crate) fn parse(raw_text: &str, type_id: TypeId) -> Result<Value, CoreError> {
    raw_text
        .parse::<f32>()
        .map(|value| Value::new(type_id, value))
        .map_err(|error| CoreError::InvalidLiteral {
            raw_text: raw_text.into(),
            type_name: "float".into(),
            message: error.to_string(),
        })
}
