use language_core::{CoreError, Value};

use crate::regex::value::get;

/// Formats a regex value as its original literal text.
pub(crate) fn format(value: &Value) -> Result<String, CoreError> {
    Ok(get(value)?.raw().to_owned())
}

#[cfg(test)]
#[path = "formatting.tests.rs"]
mod tests;
