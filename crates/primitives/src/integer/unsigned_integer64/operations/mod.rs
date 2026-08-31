mod addition_unsigned_integer64;
mod division_unsigned_integer64;
mod multiplication_unsigned_integer64;
mod power_unsigned_integer64;
mod remainder_unsigned_integer64;
mod subtraction_unsigned_integer64;

use language_core::{BinaryOperator, CoreError, Registry, TypeId};

use self::{
    addition_unsigned_integer64::addition_unsigned_integer,
    division_unsigned_integer64::division_unsigned_integer,
    multiplication_unsigned_integer64::multiplication_unsigned_integer,
    power_unsigned_integer64::power_unsigned_integer,
    remainder_unsigned_integer64::remainder_unsigned_integer,
    subtraction_unsigned_integer64::subtraction_unsigned_integer,
};

/// Registers every binary arithmetic operator for two `uint` operands.
pub(crate) fn register(
    registry: &mut Registry,
    unsigned_integer_id: TypeId,
) -> Result<(), CoreError> {
    registry.register_binary_operator(
        BinaryOperator::Addition,
        unsigned_integer_id,
        unsigned_integer_id,
        unsigned_integer_id,
        addition_unsigned_integer,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Subtraction,
        unsigned_integer_id,
        unsigned_integer_id,
        unsigned_integer_id,
        subtraction_unsigned_integer,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Multiplication,
        unsigned_integer_id,
        unsigned_integer_id,
        unsigned_integer_id,
        multiplication_unsigned_integer,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Division,
        unsigned_integer_id,
        unsigned_integer_id,
        unsigned_integer_id,
        division_unsigned_integer,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Remainder,
        unsigned_integer_id,
        unsigned_integer_id,
        unsigned_integer_id,
        remainder_unsigned_integer,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Power,
        unsigned_integer_id,
        unsigned_integer_id,
        unsigned_integer_id,
        power_unsigned_integer,
    )?;
    Ok(())
}
