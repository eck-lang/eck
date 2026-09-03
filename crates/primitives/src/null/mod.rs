mod comparisons;
mod formatting;
mod literal;
mod value;

use language_core::{CoreError, Extension, Registry, TypeDescriptor};

/// Registers the built-in null type.
pub struct NullExtension;

impl Extension for NullExtension {
    /// Returns the stable extension name used by the registry.
    fn name(&self) -> &'static str {
        "null"
    }

    /// Registers `null`, its null literal, and makes it the default type for null literals.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let id = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id,
            name: "null",
            parse_numeric_literal: None,
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: Some(literal::parse),
            format: formatting::format,
        })?;
        registry.set_default_null(id)?;
        comparisons::register(registry)
    }
}

/// Allocates a valid null type identifier for isolated tests.
#[cfg(test)]
pub(crate) fn test_type_id() -> language_core::TypeId {
    let mut registry = language_core::Registry::new();
    registry.allocate_type_id()
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
