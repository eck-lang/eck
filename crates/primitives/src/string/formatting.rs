use language_core::{CoreError, Value};

use crate::string::value::get;

/// Formats a string as its unquoted contents.
pub(crate) fn format(value: &Value) -> Result<String, CoreError> {
    Ok(get(value)?.to_owned())
}

#[cfg(test)]
#[path = "formatting.tests.rs"]
mod tests;
