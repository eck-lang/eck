use language_core::{CoreError, Value};

use super::value::get;

/// Formats a null runtime value in its canonical representation.
pub(crate) fn format(value: &Value) -> Result<String, CoreError> {
    get(value)?;
    Ok("null".into())
}

#[cfg(test)]
#[path = "formatting.tests.rs"]
mod tests;
