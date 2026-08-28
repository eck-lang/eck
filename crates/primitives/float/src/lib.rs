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

    /// Registers `float` and its arithmetic operators.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let id = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id,
            name: "float",
            parse_numeric_literal: Some(literal::parse),
            parse_string_literal: None,
            format: formatting::format,
        })?;
        operations::register(registry, id)
    }
}

/// Allocates a valid float type identifier for isolated operation tests.
#[cfg(test)]
pub(crate) fn test_type_id() -> language_core::TypeId {
    let mut registry = language_core::Registry::new();
    registry.allocate_type_id()
}

#[cfg(test)]
mod tests {
    use language_core::{CoreError, Extension, Registry};

    use super::FloatExtension;

    #[test]
    fn float_literals_use_single_precision_and_native_formatting() {
        let mut registry = Registry::new();
        FloatExtension.register(&mut registry).unwrap();
        let float = registry.type_by_name("float").unwrap();

        let value = registry.parse_numeric("16777217", Some(float)).unwrap();

        assert_eq!(*value.downcast_ref::<f32>().unwrap(), 16_777_216.0);
        assert_eq!(registry.format_value(&value).unwrap(), "16777216");
    }

    #[test]
    fn float_literals_reject_invalid_source_text() {
        let mut registry = Registry::new();
        FloatExtension.register(&mut registry).unwrap();
        let float = registry.type_by_name("float").unwrap();

        let result = registry.parse_numeric("not-a-number", Some(float));

        assert!(matches!(result, Err(CoreError::InvalidLiteral { .. })));
    }
}
