mod unsigned_integer8;

use crate::semantic::{CoreError, Registry};

use crate::primitives::comparison::{declare_pair, evaluate_total_order as evaluate};

/// Registers the unsigned integer comparison relation.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    unsigned_integer8::register(registry)
}
