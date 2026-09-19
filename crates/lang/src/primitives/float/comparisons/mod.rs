mod float;
mod float_integer;

use crate::semantic::{CoreError, Registry};

use crate::primitives::comparison::{declare_pair, evaluate_partial_order as evaluate};

/// Registers float comparison relations and their mixed-type overloads.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    float::register(registry)?;
    float_integer::register(registry)?;

    Ok(())
}
