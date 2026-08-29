mod multiplication_boolean;

use language_core::{BinaryOperator, CoreError, Registry, TypeId};

use self::multiplication_boolean::{
    multiplication_boolean_numeric, multiplication_numeric_boolean,
};

const NUMERIC_TYPE_NAMES: &[&str] = &["int", "float", "double", "decimal"];

/// Registers multiplication between `bool` and each installed built-in numeric type.
pub(crate) fn register(registry: &mut Registry, boolean: TypeId) -> Result<(), CoreError> {
    for numeric_name in NUMERIC_TYPE_NAMES {
        let Some(numeric) = registry.type_by_name(numeric_name) else {
            continue;
        };
        registry.register_binary_operator(
            BinaryOperator::Multiplication,
            numeric,
            boolean,
            numeric,
            multiplication_numeric_boolean,
        )?;
        registry.register_binary_operator(
            BinaryOperator::Multiplication,
            boolean,
            numeric,
            numeric,
            multiplication_boolean_numeric,
        )?;
    }
    Ok(())
}
