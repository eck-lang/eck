mod integer8;

use language_core::{CoreError, Registry};

use crate::comparison::{declare_pair, evaluate_total_order as evaluate};

/// Registers the integer comparison relation.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    integer8::register(registry)
}
