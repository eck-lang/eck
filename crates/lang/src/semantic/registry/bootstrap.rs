//! Composition of the built-in ECK language domains.

use crate::containers::array::ArrayExtension;
use crate::measures::MeasuresExtension;
use crate::semantic::{CoreError, Extension, Registry};
use crate::std::io::IoExtension;

/// Builds a fresh registry containing all built-in ECK extensions.
pub fn default_registry() -> Result<Registry, CoreError> {
    let mut registry = Registry::new();
    register_all(&mut registry)?;
    Ok(registry)
}

/// Registers every built-in extension into an existing registry.
pub fn register_all(registry: &mut Registry) -> Result<(), CoreError> {
    crate::primitives::register_all(registry)?;
    MeasuresExtension.register(registry)?;
    ArrayExtension.register(registry)?;
    IoExtension.register(registry)?;
    Ok(())
}
