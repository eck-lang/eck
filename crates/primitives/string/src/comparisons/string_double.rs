use language_core::{CoreError, Registry};

use super::declare_distinct_pair;

/// Registers strict equality and inequality between strings and doubles.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    declare_distinct_pair::<f64>(registry, "double")
}

#[cfg(test)]
#[path = "string_double.tests.rs"]
mod tests;
