use std::sync::Arc;

use language_core::{CoreError, Value};
use regex::Regex;

/// The compiled regular expression stored as a runtime value.
///
/// The value preserves the original literal text for formatting while holding
/// an efficiently reusable compiled `Regex`. `is_global` controls whether
/// string operations replace all occurrences or only the first.
#[derive(Debug, Clone)]
pub struct RegexValue {
    raw: String,
    is_global: bool,
    regex: Arc<Regex>,
}

impl RegexValue {
    /// Creates a new regex value from its components.
    pub(crate) fn new(raw: String, is_global: bool, regex: Regex) -> Self {
        Self {
            raw,
            is_global,
            regex: Arc::new(regex),
        }
    }

    /// Returns the original literal text, including slashes and flags.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns whether the `g` flag was present.
    pub fn is_global(&self) -> bool {
        self.is_global
    }

    /// Returns the compiled regular expression.
    pub fn regex(&self) -> &Regex {
        &self.regex
    }
}

/// Extracts the regex payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<&RegexValue, CoreError> {
    value
        .downcast_ref::<RegexValue>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("regex".into()))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
