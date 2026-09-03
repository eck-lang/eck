mod addition_integer64;
mod division_integer64;
mod multiplication_integer64;
mod power_integer64;
mod remainder_integer64;
mod subtraction_integer64;

use language_core::{BinaryOperator, BinaryOperatorExecutor, CoreError, Registry, TypeId};

use self::{
    addition_integer64::{addition_integer, addition_mixed_integer},
    division_integer64::{division_integer, division_mixed_integer},
    multiplication_integer64::{multiplication_integer, multiplication_mixed_integer},
    power_integer64::{power_integer, power_mixed_integer},
    remainder_integer64::{remainder_integer, remainder_mixed_integer},
    subtraction_integer64::{subtraction_integer, subtraction_mixed_integer},
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

/// Registers every mixed-width operator whose static result is `int64`.
pub(crate) fn register_promotions(
    registry: &mut Registry,
    narrower_ids: &[TypeId],
    integer64_id: TypeId,
) -> Result<(), CoreError> {
    for narrower_id in narrower_ids {
        for (operator, execute) in [
            (
                BinaryOperator::Addition,
                addition_mixed_integer as BinaryOperatorExecutor,
            ),
            (BinaryOperator::Subtraction, subtraction_mixed_integer),
            (BinaryOperator::Multiplication, multiplication_mixed_integer),
            (BinaryOperator::Division, division_mixed_integer),
            (BinaryOperator::Remainder, remainder_mixed_integer),
            (BinaryOperator::Power, power_mixed_integer),
        ] {
            register_mixed_pair(registry, operator, *narrower_id, integer64_id, execute)?;
        }
    }
    Ok(())
}

/// Registers one mixed-width operator in both source orders with the wider result.
fn register_mixed_pair(
    registry: &mut Registry,
    operator: BinaryOperator,
    narrower_id: TypeId,
    wider_id: TypeId,
    execute: BinaryOperatorExecutor,
) -> Result<(), CoreError> {
    registry.register_binary_operator(operator, narrower_id, wider_id, wider_id, execute)?;
    registry.register_binary_operator(operator, wider_id, narrower_id, wider_id, execute)?;
    Ok(())
}
