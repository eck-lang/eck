use language_core::{CoreError, Registry};
use rust_decimal::Decimal;

use super::declare_distinct_pair;

/// Registers strict equality and inequality between strings and decimals.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    declare_distinct_pair::<Decimal>(registry, "decimal")
}

#[cfg(test)]
#[path = "string_decimal.tests.rs"]
mod tests;
