pub(crate) mod formatting;
pub(crate) mod literal;
pub(crate) mod value;

use language_core::{CoreError, Extension, Registry, TypeDescriptor};

/// Registers the built-in regex type and its literal semantics.
pub struct RegexExtension;

impl Extension for RegexExtension {
    /// Returns the stable extension name used by the registry.
    fn name(&self) -> &'static str {
        "regex"
    }

    /// Registers `regex` and its formatting.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let regex_type = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id: regex_type,
            name: "regex",
            parse_numeric_literal: None,
            parse_string_literal: None,
            parse_regex_literal: Some(literal::parse),
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: formatting::format,
        })?;
        registry.set_default_regex(regex_type)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
