mod null;

use language_core::{CoreError, Registry};

/// Registers the null equality relations.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    self::null::register(registry)
}
