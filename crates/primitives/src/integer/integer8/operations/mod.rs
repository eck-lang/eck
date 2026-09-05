mod addition_integer8;
mod division_integer8;
mod multiplication_integer8;
mod power_integer8;
mod remainder_integer8;
mod subtraction_integer8;

use language_core::{BinaryOperator, CoreError, Registry, TypeId};

use self::{
    addition_integer8::{addition_integer, addition_integer_with_context},
    division_integer8::{division_integer, division_integer_with_context},
    multiplication_integer8::{multiplication_integer, multiplication_integer_with_context},
    power_integer8::{power_integer, power_integer_with_context},
    remainder_integer8::{remainder_integer, remainder_integer_with_context},
    subtraction_integer8::{subtraction_integer, subtraction_integer_with_context},
};

/// Registers every binary arithmetic operator for two `int8` operands.
///
/// Same-type operators keep `int8` as their static result while their
/// context-aware override promotes overflowed results to `int16` at runtime.
pub(crate) fn register(registry: &mut Registry, integer_id: TypeId) -> Result<(), CoreError> {
    registry.register_context_binary_operator(
        BinaryOperator::Addition,
        integer_id,
        integer_id,
        integer_id,
        addition_integer,
        addition_integer_with_context,
    )?;
    registry.register_context_binary_operator(
        BinaryOperator::Subtraction,
        integer_id,
        integer_id,
        integer_id,
        subtraction_integer,
        subtraction_integer_with_context,
    )?;
    registry.register_context_binary_operator(
        BinaryOperator::Multiplication,
        integer_id,
        integer_id,
        integer_id,
        multiplication_integer,
        multiplication_integer_with_context,
    )?;
    registry.register_context_binary_operator(
        BinaryOperator::Division,
        integer_id,
        integer_id,
        integer_id,
        division_integer,
        division_integer_with_context,
    )?;
    registry.register_context_binary_operator(
        BinaryOperator::Remainder,
        integer_id,
        integer_id,
        integer_id,
        remainder_integer,
        remainder_integer_with_context,
    )?;
    registry.register_context_binary_operator(
        BinaryOperator::Power,
        integer_id,
        integer_id,
        integer_id,
        power_integer,
        power_integer_with_context,
    )?;
    Ok(())
}
