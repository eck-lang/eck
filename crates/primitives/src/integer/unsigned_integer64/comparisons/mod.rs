mod unsigned_integer64;

use language_core::{CoreError, Registry};

use crate::comparison::{declare_pair, evaluate_total_order as evaluate};

/// Registers the unsigned integer comparison relation.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    unsigned_integer64::register(registry)
}
