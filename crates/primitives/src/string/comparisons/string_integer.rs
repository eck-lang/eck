use language_core::{CoreError, Registry};

use super::declare_distinct_pair;

/// Registers strict equality and inequality between strings and integers.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    declare_distinct_pair::<i64>(registry, "int")
}

#[cfg(test)]
#[path = "string_integer.tests.rs"]
mod tests;
