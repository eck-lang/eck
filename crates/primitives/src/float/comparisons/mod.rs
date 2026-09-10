mod float;
mod float_integer;

use language_core::{CoreError, Registry};

use crate::comparison::{declare_pair, evaluate_partial_order as evaluate};

/// Registers float comparison relations and their mixed-type overloads.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    float::register(registry)?;
    float_integer::register(registry)?;

    Ok(())
}
