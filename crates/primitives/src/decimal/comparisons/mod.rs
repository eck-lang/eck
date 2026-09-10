mod binary_float;
mod decimal;
mod decimal_double;
mod decimal_float;
mod decimal_integer;

use language_core::{CoreError, Registry};

use crate::comparison::{declare_pair, evaluate_partial_order as evaluate};

/// Registers decimal comparison relations and their mixed-type overloads.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    decimal::register(registry)?;
    decimal_integer::register(registry)?;
    decimal_float::register(registry)?;
    decimal_double::register(registry)?;

    Ok(())
}
