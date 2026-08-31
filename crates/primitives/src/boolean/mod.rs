mod comparisons;
mod formatting;
mod literal;
mod operations;
mod value;

use language_core::{CoreError, Extension, Registry, TypeDescriptor};

/// Registers the built-in boolean type.
pub struct BoolExtension;

impl Extension for BoolExtension {
    /// Returns the stable extension name used by the registry.
    fn name(&self) -> &'static str {
        "bool"
    }

    /// Registers `bool`, its multiplication with installed numeric primitives,
    /// and makes it the default type for boolean literals.
    ///
    /// Register the numeric primitive extensions before this extension so their
    /// boolean multiplication operators are available.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let id = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id,
            name: "bool",
            parse_numeric_literal: None,
            parse_string_literal: None,
            parse_boolean_literal: Some(literal::parse),
            format: formatting::format,
        })?;
        registry.set_default_boolean(id, value::get)?;
        operations::register(registry, id)?;
        comparisons::register(registry)
    }
}

/// Allocates a valid boolean type identifier for isolated tests.
#[cfg(test)]
pub(crate) fn test_type_id() -> language_core::TypeId {
    let mut registry = language_core::Registry::new();
    registry.allocate_type_id()
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
