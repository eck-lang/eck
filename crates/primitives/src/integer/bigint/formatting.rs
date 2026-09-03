use language_core::{CoreError, Value};

use super::value::get;

/// Formats an arbitrary-precision integer runtime value in its canonical decimal representation.
pub(crate) fn format(value: &Value) -> Result<String, CoreError> {
    Ok(get(value)?.to_string())
}

#[cfg(test)]
#[path = "formatting.tests.rs"]
mod tests;
