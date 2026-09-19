mod boolean;

use crate::semantic::{CoreError, Registry};

/// Registers the boolean equality relation.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    boolean::register(registry)
}
