mod bigint;
mod mixed;

use crate::semantic::{CoreError, Registry};

use crate::primitives::comparison::{declare_pair, evaluate_total_order as evaluate};

/// Registers the integer comparison relation.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    bigint::register(registry)
}

/// Declares every mixed-width comparison whose wider operand is `bigint`.
pub(crate) fn register_promotions(registry: &mut Registry) -> Result<(), CoreError> {
    mixed::register(registry)
}
