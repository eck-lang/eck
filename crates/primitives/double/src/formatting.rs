use language_core::{CoreError, Value};

use super::value::get;

/// Formats a double runtime value in Rust's canonical representation.
pub(crate) fn format(value: &Value) -> Result<String, CoreError> {
    Ok(get(value)?.to_string())
}
