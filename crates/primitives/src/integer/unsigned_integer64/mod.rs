mod comparisons;
mod formatting;
mod literal;
pub(crate) mod operations;
mod value;

use language_core::{CoreError, Extension, Registry, TypeDescriptor};

/// Registers the built-in unsigned 64-bit integer type and its operators.
pub struct UnsignedIntegerExtension;

impl UnsignedIntegerExtension {
    /// Creates an extension for the unsigned 64-bit integer type with alias support.
    pub const fn new() -> Self {
        Self
    }
}

/// Backwards-compatible alias for the unsigned 64-bit integer extension.
pub type UnsignedInteger64Extension = UnsignedIntegerExtension;

impl Extension for UnsignedIntegerExtension {
    /// Returns the stable extension name used by the registry.
    fn name(&self) -> &'static str {
        "uint64"
    }

    /// Registers `uint64` (with alias `uint`) and its numeric semantics.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let id = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id,
            name: "uint64",
            parse_numeric_literal: Some(literal::parse),
            parse_string_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: formatting::format,
        })?;
        registry.register_type_alias("uint", id)?;
        operations::register(registry, id)?;
        comparisons::register(registry)
    }
}

/// Allocates a valid unsigned integer type identifier for isolated operation tests.
#[cfg(test)]
pub(crate) fn test_type_id() -> language_core::TypeId {
    let mut registry = language_core::Registry::new();
    registry.allocate_type_id()
}
#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
