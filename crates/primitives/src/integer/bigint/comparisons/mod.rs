mod bigint;
mod mixed;

use language_core::{CoreError, Registry};

use crate::comparison::{declare_pair, evaluate_total_order as evaluate};

/// Registers the integer comparison relation.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    bigint::register(registry)
}

/// Declares every mixed-width comparison whose wider operand is `bigint`.
pub(crate) fn register_promotions(registry: &mut Registry) -> Result<(), CoreError> {
    mixed::register(registry)
}
