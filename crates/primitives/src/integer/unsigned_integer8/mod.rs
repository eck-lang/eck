mod comparisons;
mod formatting;
mod literal;
pub(crate) mod operations;
mod value;

use language_core::{CoreError, Extension, Registry, TypeDescriptor};

/// Registers the built-in unsigned 8-bit integer type and its operators.
pub struct UnsignedInteger8Extension;

impl UnsignedInteger8Extension {
    /// Creates an extension for the unsigned 8-bit integer type.
    pub const fn new() -> Self {
        Self
    }
}

impl Extension for UnsignedInteger8Extension {
    /// Returns the stable extension name used by the registry.
    fn name(&self) -> &'static str {
        "uint8"
    }

    /// Registers `uint8` and its numeric semantics.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let id = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id,
            name: "uint8",
            parse_numeric_literal: Some(literal::parse),
            parse_string_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: formatting::format,
        })?;
        operations::register(registry, id)?;
        comparisons::register(registry)
    }
}

/// Allocates a valid unsigned 8-bit integer type identifier for isolated operation tests.
#[cfg(test)]
pub(crate) fn test_type_id() -> language_core::TypeId {
    let mut registry = language_core::Registry::new();
    registry.allocate_type_id()
}
#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
