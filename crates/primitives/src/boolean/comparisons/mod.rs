mod boolean;

use language_core::{CoreError, Registry};

/// Registers the boolean equality relation.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    boolean::register(registry)
}
