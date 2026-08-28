mod addition_float;
mod division_float;
mod multiplication_float;
mod power_float;
mod remainder_float;
mod subtraction_float;

use language_core::{BinaryOperator, CoreError, Registry, TypeId};

use self::{
    addition_float::addition_float, division_float::division_float,
    multiplication_float::multiplication_float, power_float::power_float,
    remainder_float::remainder_float, subtraction_float::subtraction_float,
};

/// Registers every binary arithmetic operator for two `float` operands.
pub(crate) fn register(registry: &mut Registry, float_id: TypeId) -> Result<(), CoreError> {
    registry.register_binary_operator(
        BinaryOperator::Addition,
        float_id,
        float_id,
        float_id,
        addition_float,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Subtraction,
        float_id,
        float_id,
        float_id,
        subtraction_float,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Multiplication,
        float_id,
        float_id,
        float_id,
        multiplication_float,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Division,
        float_id,
        float_id,
        float_id,
        division_float,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Remainder,
        float_id,
        float_id,
        float_id,
        remainder_float,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Power,
        float_id,
        float_id,
        float_id,
        power_float,
    )?;
    Ok(())
}
