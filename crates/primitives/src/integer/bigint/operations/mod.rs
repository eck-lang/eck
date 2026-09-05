mod addition_bigint;
mod division_bigint;
mod multiplication_bigint;
mod power_bigint;
mod remainder_bigint;
mod subtraction_bigint;

use language_core::{BinaryOperator, BinaryOperatorExecutor, CoreError, Registry, TypeId};

use self::{
    addition_bigint::addition_integer, division_bigint::division_integer,
    multiplication_bigint::multiplication_integer, power_bigint::power_integer,
    remainder_bigint::remainder_integer, subtraction_bigint::subtraction_integer,
};
use self::{
    addition_bigint::addition_mixed_integer, division_bigint::division_mixed_integer,
    multiplication_bigint::multiplication_mixed_integer, power_bigint::power_mixed_integer,
    remainder_bigint::remainder_mixed_integer, subtraction_bigint::subtraction_mixed_integer,
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

/// Registers every mixed-width operator whose static result is `bigint`.
pub(crate) fn register_promotions(
    registry: &mut Registry,
    narrower_ids: &[TypeId],
    bigint_id: TypeId,
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
            register_mixed_pair(registry, operator, *narrower_id, bigint_id, execute)?;
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
