mod addition_integer;
mod division_integer;
mod multiplication_integer;
mod power_integer;
mod remainder_integer;
mod subtraction_integer;

use language_core::{BinaryOperator, CoreError, Registry, TypeId};

use self::{
    addition_integer::addition_integer, division_integer::division_integer,
    multiplication_integer::multiplication_integer, power_integer::power_integer,
    remainder_integer::remainder_integer, subtraction_integer::subtraction_integer,
};

/// Registers every binary arithmetic operator for two `int` operands.
pub(crate) fn register(registry: &mut Registry, integer_id: TypeId) -> Result<(), CoreError> {
    registry.register_binary_operator(
        BinaryOperator::Addition,
        integer_id,
        integer_id,
        integer_id,
        addition_integer,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Subtraction,
        integer_id,
        integer_id,
        integer_id,
        subtraction_integer,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Multiplication,
        integer_id,
        integer_id,
        integer_id,
        multiplication_integer,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Division,
        integer_id,
        integer_id,
        integer_id,
        division_integer,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Remainder,
        integer_id,
        integer_id,
        integer_id,
        remainder_integer,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Power,
        integer_id,
        integer_id,
        integer_id,
        power_integer,
    )?;
    Ok(())
}
