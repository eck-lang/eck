mod addition_double;
mod division_double;
mod multiplication_double;
mod power_double;
mod remainder_double;
mod subtraction_double;

use language_core::{BinaryOperator, CoreError, Registry, TypeId};

use self::{
    addition_double::addition_double, division_double::division_double,
    multiplication_double::multiplication_double, power_double::power_double,
    remainder_double::remainder_double, subtraction_double::subtraction_double,
};

/// Registers every binary arithmetic operator for two `double` operands.
pub(crate) fn register(registry: &mut Registry, double_id: TypeId) -> Result<(), CoreError> {
    registry.register_binary_operator(
        BinaryOperator::Addition,
        double_id,
        double_id,
        double_id,
        addition_double,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Subtraction,
        double_id,
        double_id,
        double_id,
        subtraction_double,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Multiplication,
        double_id,
        double_id,
        double_id,
        multiplication_double,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Division,
        double_id,
        double_id,
        double_id,
        division_double,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Remainder,
        double_id,
        double_id,
        double_id,
        remainder_double,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Power,
        double_id,
        double_id,
        double_id,
        power_double,
    )?;
    Ok(())
}
