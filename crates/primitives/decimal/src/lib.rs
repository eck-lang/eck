mod formatting;
mod literal;
pub(crate) mod operations;
pub(crate) mod value;

use language_core::{CoreError, Extension, Registry, TypeDescriptor};

/// Registers the built-in fixed-precision decimal type and its operators.
pub struct DecimalExtension;

impl Extension for DecimalExtension {
    /// Returns the stable extension name used by the registry.
    fn name(&self) -> &'static str {
        "decimal"
    }

    /// Registers `decimal`, makes it the default fractional type, and adds its operators.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let id = registry.allocate_type_id();
        registry.register_type(TypeDescriptor {
            id,
            name: "decimal",
            parse_numeric_literal: Some(literal::parse),
            parse_string_literal: None,
            format: formatting::format,
        })?;
        registry.set_default_fractional(id)?;
        operations::register(registry, id)
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
mod tests {
    use double::DoubleExtension;
    use float::FloatExtension;
    use language_core::{BinaryOperator, Extension, Registry, ValueType};

    use super::DecimalExtension;

    #[test]
    fn decimal_is_the_default_fractional_type() {
        let mut registry = Registry::new();
        DecimalExtension.register(&mut registry).unwrap();

        let value = registry.parse_numeric("0.2", None).unwrap();

        assert_eq!(value.type_id(), registry.type_by_name("decimal").unwrap());
    }

    #[test]
    fn decimal_registers_all_mixed_float_and_double_operators() {
        let mut registry = Registry::new();
        FloatExtension.register(&mut registry).unwrap();
        DoubleExtension.register(&mut registry).unwrap();
        DecimalExtension.register(&mut registry).unwrap();

        let decimal = registry.type_by_name("decimal").unwrap();
        let float = registry.type_by_name("float").unwrap();
        let double = registry.type_by_name("double").unwrap();

        for operator in [
            BinaryOperator::Addition,
            BinaryOperator::Subtraction,
            BinaryOperator::Multiplication,
            BinaryOperator::Division,
            BinaryOperator::Remainder,
            BinaryOperator::Power,
        ] {
            assert_eq!(
                registry
                    .resolve_binary_operation(
                        operator,
                        ValueType::plain(decimal),
                        ValueType::plain(float),
                    )
                    .unwrap()
                    .output,
                ValueType::plain(decimal)
            );
            assert_eq!(
                registry
                    .resolve_binary_operation(
                        operator,
                        ValueType::plain(decimal),
                        ValueType::plain(double),
                    )
                    .unwrap()
                    .output,
                ValueType::plain(decimal)
            );
        }
    }
}
