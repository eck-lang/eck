pub(crate) mod comparisons;
mod formatting;
mod literal;
pub(crate) mod operations;
mod value;

use crate::semantic::{CoreError, Extension, Registry, TypeDescriptor};

/// Registers the built-in signed 16-bit integer type and its operators.
pub struct Integer16Extension;

impl Integer16Extension {
    /// Creates an extension for the signed 16-bit integer type.
    pub const fn new() -> Self {
        Self
    }
}

impl Extension for Integer16Extension {
    /// Returns the stable extension name used by the registry.
    fn name(&self) -> &'static str {
        "int16"
    }

    /// Registers `int16` and its numeric semantics.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let id = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id,
            name: "int16",
            is_integer: true,
            parse_numeric_literal: Some(literal::parse),
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: formatting::format,
        })?;
        registry.register_index_extractor(id, value::to_index)?;
        operations::register(registry, id)?;
        comparisons::register(registry)
    }
}

/// Allocates a valid signed 16-bit integer type identifier for isolated operation tests.
#[cfg(test)]
pub(crate) fn test_type_id() -> crate::semantic::TypeId {
    let mut registry = crate::semantic::Registry::new();
    registry.allocate_type_id()
}
#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
