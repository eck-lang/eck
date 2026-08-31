use language_core::{CoreError, TypeId, Value};

use super::value::Null;

/// Parses source text into a null value.
pub(crate) fn parse(raw_text: &str, type_id: TypeId) -> Result<Value, CoreError> {
    match raw_text {
        "null" => Ok(Value::new(type_id, Null)),
        _ => Err(CoreError::InvalidLiteral {
            raw_text: raw_text.into(),
            type_name: "null".into(),
            message: "expected `null`".into(),
        }),
    }
}

#[cfg(test)]
#[path = "literal.tests.rs"]
mod tests;
