use language_core::{CoreError, TypeId, Value};

/// Stores decoded source text as a Unicode string runtime value.
pub(crate) fn parse(decoded_text: &str, string_type: TypeId) -> Result<Value, CoreError> {
    Ok(Value::new(string_type, decoded_text.to_owned()))
}

#[cfg(test)]
#[path = "literal.tests.rs"]
mod tests;
