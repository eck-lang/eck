use language_core::{CoreError, Extension, Registry};
use measures::MeasuresExtension;
use percentage::PercentageExtension;

/// Re-exported keyword list from the language crate – the parser remains the
/// single source of truth. `eck-dialect` re-exports it for convenience so
/// consumers can depend only on the dialect.
pub use parser::ECK_KEYWORDS;

/// Builds a fresh `Registry` with every builtin ECK extension registered.
///
/// The order is deterministic and preserves the existing CLI/LSP dialect.
/// Adding a new primitive or measure extension requires a single change here;
/// all consumers (CLI, LSP, tests) automatically see the new type.
pub fn default_registry() -> Result<Registry, CoreError> {
    let mut registry = Registry::new();
    register_all(&mut registry)?;
    Ok(registry)
}

/// Registers every builtin extension into an existing `Registry`.
///
/// This is useful when the caller already owns a `Registry` and wants to
/// populate it without replacing it.
pub fn register_all(registry: &mut Registry) -> Result<(), CoreError> {
    primitives::register_all(registry)?;
    MeasuresExtension.register(registry)?;
    PercentageExtension.register(registry)?;
    io_input_output::IoExtension.register(registry)?;
    Ok(())
}

#[cfg(test)]
#[path = "lib.tests.rs"]
mod tests;
