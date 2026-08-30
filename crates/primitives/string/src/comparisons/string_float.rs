use language_core::{CoreError, Registry};

use super::declare_distinct_pair;

/// Registers strict equality and inequality between strings and floats.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    declare_distinct_pair::<f32>(registry, "float")
}

#[cfg(test)]
#[path = "string_float.tests.rs"]
mod tests;
