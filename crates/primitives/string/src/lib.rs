mod comparisons;
mod conversion;
mod formatting;
mod literal;
mod operations;
mod value;

use language_core::{CoreError, Extension, FunctionSignature, Registry, TypeDescriptor};

/// Registers the built-in Unicode string type and its language semantics.
pub struct StringExtension;

impl Extension for StringExtension {
    /// Returns the stable extension name used by the registry.
    fn name(&self) -> &'static str {
        "string"
    }

    /// Registers `string`, its operators, comparisons, and formatting conversion.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let string_type = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id: string_type,
            name: "string",
            parse_numeric_literal: None,
            parse_string_literal: Some(literal::parse),
            parse_boolean_literal: None,
            format: formatting::format,
        })?;
        registry.set_default_string(string_type)?;
        operations::register(registry, string_type)?;
        comparisons::register(registry)?;
        registry.register_function(
            "string",
            FunctionSignature::AnySingle,
            Some(string_type),
            conversion::format_as_string,
        )?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "lib.tests.rs"]
mod tests;
