mod double;
mod double_float;
mod double_integer;

use crate::semantic::{CoreError, Registry};

use crate::primitives::comparison::{declare_pair, evaluate_partial_order as evaluate};

/// Registers double comparison relations and their mixed-type overloads.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    double::register(registry)?;
    double_float::register(registry)?;
    double_integer::register(registry)?;

    Ok(())
}
