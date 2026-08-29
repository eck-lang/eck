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
        registry.set_default_boolean(id)?;
        operations::register(registry, id)
    }
}

/// Allocates a valid boolean type identifier for isolated tests.
#[cfg(test)]
pub(crate) fn test_type_id() -> language_core::TypeId {
    let mut registry = language_core::Registry::new();
    registry.allocate_type_id()
}

#[cfg(test)]
mod tests {
    use decimal::DecimalExtension;
    use double::DoubleExtension;
    use float::FloatExtension;
    use integer::IntegerExtension;
    use language_core::{BinaryOperator, CoreError, Extension, Registry, ValueType};

    use super::BoolExtension;

    #[test]
    fn bool_is_the_default_boolean_type() {
        let mut registry = Registry::new();
        BoolExtension.register(&mut registry).unwrap();

        let value = registry.parse_boolean("true", None).unwrap();

        assert_eq!(value.type_id(), registry.type_by_name("bool").unwrap());
        assert!(*value.downcast_ref::<bool>().unwrap());
    }

    #[test]
    fn bool_literals_reject_invalid_source_text() {
        let mut registry = Registry::new();
        BoolExtension.register(&mut registry).unwrap();

        let result = registry.parse_boolean("yes", None);

        assert!(matches!(result, Err(CoreError::InvalidLiteral { .. })));
    }

    #[test]
    fn registers_multiplication_with_every_installed_numeric_primitive() {
        let mut registry = Registry::new();
        IntegerExtension.register(&mut registry).unwrap();
        FloatExtension.register(&mut registry).unwrap();
        DoubleExtension.register(&mut registry).unwrap();
        DecimalExtension.register(&mut registry).unwrap();
        BoolExtension.register(&mut registry).unwrap();
        let boolean = registry.type_by_name("bool").unwrap();

        for numeric_name in ["int", "float", "double", "decimal"] {
            let numeric = registry.type_by_name(numeric_name).unwrap();
            for (left, right) in [(numeric, boolean), (boolean, numeric)] {
                assert_eq!(
                    registry
                        .resolve_binary_operation(
                            BinaryOperator::Multiplication,
                            ValueType::plain(left),
                            ValueType::plain(right),
                        )
                        .unwrap()
                        .output,
                    ValueType::plain(numeric)
                );
            }
        }
    }
}
