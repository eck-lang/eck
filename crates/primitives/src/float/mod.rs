mod comparisons;
mod formatting;
mod literal;
pub(crate) mod operations;
mod value;

use language_core::{CoreError, Extension, Registry, TypeDescriptor};

/// Registers the built-in single-precision floating-point type and its operators.
pub struct FloatExtension;

impl Extension for FloatExtension {
    /// Returns the stable extension name used by the registry.
    fn name(&self) -> &'static str {
        "float"
    }

    /// Registers `float`, its arithmetic operators, and numeric comparisons.
    ///
    /// Mixed arithmetic with `int` produces `float`; integer magnitudes outside
    /// exact single-precision representation may be rounded during conversion.
    /// Comparison declarations with `int` activate independently of extension
    /// registration order.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let id = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id,
            name: "float",
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

/// Allocates a valid float type identifier for isolated operation tests.
#[cfg(test)]
pub(crate) fn test_type_id() -> language_core::TypeId {
    let mut registry = language_core::Registry::new();
    registry.allocate_type_id()
}
#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
