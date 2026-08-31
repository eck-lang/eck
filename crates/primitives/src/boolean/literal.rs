use language_core::{CoreError, TypeId, Value};

/// Parses source text into a boolean value.
pub(crate) fn parse(raw_text: &str, type_id: TypeId) -> Result<Value, CoreError> {
    match raw_text {
        "true" => Ok(Value::new(type_id, true)),
        "false" => Ok(Value::new(type_id, false)),
        _ => Err(CoreError::InvalidLiteral {
            raw_text: raw_text.into(),
            type_name: "bool".into(),
            message: "expected `true` or `false`".into(),
        }),
    }
}

#[cfg(test)]
#[path = "literal.tests.rs"]
mod tests;
