mod comparisons;
mod formatting;
mod literal;
pub(crate) mod operations;
pub(crate) mod value;

use language_core::{CoreError, Extension, Registry, TypeDescriptor};

/// Registers the built-in fixed-precision decimal type and its semantics.
pub struct DecimalExtension;

impl Extension for DecimalExtension {
    /// Returns the stable extension name used by the registry.
    fn name(&self) -> &'static str {
        "decimal"
    }

    /// Registers `decimal`, makes it the default fractional type, and adds its
    /// arithmetic and comparison relations.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let id = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id,
            name: "decimal",
            parse_numeric_literal: Some(literal::parse),
            parse_string_literal: None,
            parse_boolean_literal: None,
            format: formatting::format,
        })?;
        registry.set_default_fractional(id)?;
        operations::register(registry, id)?;
        comparisons::register(registry)
    }
}

/// Allocates valid decimal type identifiers for isolated tests.
#[cfg(test)]
pub(crate) fn test_type_id(index: u32) -> language_core::TypeId {
    use std::sync::OnceLock;

    static IDS: OnceLock<[language_core::TypeId; 8]> = OnceLock::new();
    let ids = IDS.get_or_init(|| {
        let mut registry = language_core::Registry::new();
        std::array::from_fn(|_| registry.allocate_type_id())
    });
    ids[index as usize]
}

#[cfg(test)]
#[path = "lib.tests.rs"]
mod tests;
