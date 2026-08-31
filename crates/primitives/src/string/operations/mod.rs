mod addition_string;
mod multiplication_string_integer;

use language_core::{BinaryOperator, CoreError, Registry, TypeId};

use self::{
    addition_string::addition_string, multiplication_string_integer::multiplication_string_integer,
};

/// Registers string concatenation and repetition by an installed integer type.
pub(crate) fn register(registry: &mut Registry, string_type: TypeId) -> Result<(), CoreError> {
    registry.register_binary_operator(
        BinaryOperator::Addition,
        string_type,
        string_type,
        string_type,
        addition_string,
    )?;

    if let Some(integer_type) = registry.type_by_name("int") {
        registry.register_binary_operator(
            BinaryOperator::Multiplication,
            string_type,
            integer_type,
            string_type,
            multiplication_string_integer,
        )?;
    }
    Ok(())
}
