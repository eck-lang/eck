mod comparisons;
mod formatting;
mod literal;
pub(crate) mod operations;
mod value;

use language_core::{CoreError, Extension, Registry, TypeDescriptor};

/// Registers the built-in signed 64-bit integer type and its operators.
pub struct IntegerExtension;

impl Extension for IntegerExtension {
    /// Returns the stable extension name used by the registry.
    fn name(&self) -> &'static str {
        "int"
    }

    /// Registers `int`, makes it the default integer type, and adds its numeric semantics.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let id = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id,
            name: "int",
            parse_numeric_literal: Some(literal::parse),
            parse_string_literal: None,
            parse_boolean_literal: None,
            format: formatting::format,
        })?;
        registry.set_default_integer(id)?;
        operations::register(registry, id)?;
        comparisons::register(registry)
    }
}

/// Allocates a valid integer type identifier for isolated operation tests.
#[cfg(test)]
pub(crate) fn test_type_id() -> language_core::TypeId {
    let mut registry = language_core::Registry::new();
    registry.allocate_type_id()
}
#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
