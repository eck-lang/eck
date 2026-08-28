mod formatting;
mod literal;
pub(crate) mod operations;
mod value;

use language_core::{CoreError, Extension, Registry, TypeDescriptor};

/// Registers the built-in double-precision floating-point type and its operators.
pub struct DoubleExtension;

impl Extension for DoubleExtension {
    /// Returns the stable extension name used by the registry.
    fn name(&self) -> &'static str {
        "double"
    }

    /// Registers `double` and its arithmetic operators.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let id = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id,
            name: "double",
            parse_numeric_literal: Some(literal::parse),
            parse_string_literal: None,
            format: formatting::format,
        })?;
        operations::register(registry, id)
    }
}

/// Allocates a valid double type identifier for isolated operation tests.
#[cfg(test)]
pub(crate) fn test_type_id() -> language_core::TypeId {
    let mut registry = language_core::Registry::new();
    registry.allocate_type_id()
}

#[cfg(test)]
mod tests {
    use language_core::{CoreError, Extension, Registry};

    use super::DoubleExtension;

    #[test]
    fn double_literals_preserve_double_precision_and_native_formatting() {
        let mut registry = Registry::new();
        DoubleExtension.register(&mut registry).unwrap();
        let double = registry.type_by_name("double").unwrap();

        let value = registry.parse_numeric("16777217", Some(double)).unwrap();

        assert_eq!(*value.downcast_ref::<f64>().unwrap(), 16_777_217.0);
        assert_eq!(registry.format_value(&value).unwrap(), "16777217");
    }

    #[test]
    fn double_literals_reject_invalid_source_text() {
        let mut registry = Registry::new();
        DoubleExtension.register(&mut registry).unwrap();
        let double = registry.type_by_name("double").unwrap();

        let result = registry.parse_numeric("not-a-number", Some(double));

        assert!(matches!(result, Err(CoreError::InvalidLiteral { .. })));
    }
}
