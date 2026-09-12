mod addition_integer64;
mod division_integer64;
mod multiplication_integer64;
mod power_integer64;
mod remainder_integer64;
mod subtraction_integer64;

use language_core::{BinaryOperator, ContextBinaryOperatorExecutor, CoreError, Registry, TypeId};

use self::{
    addition_integer64::{
        addition_integer, addition_integer_with_context, addition_mixed_integer,
        addition_mixed_integer_with_context,
    },
    division_integer64::{
        division_integer, division_integer_with_context, division_mixed_integer,
        division_mixed_integer_with_context,
    },
    multiplication_integer64::{
        multiplication_integer, multiplication_integer_with_context, multiplication_mixed_integer,
        multiplication_mixed_integer_with_context,
    },
    power_integer64::{
        power_integer, power_integer_with_context, power_mixed_integer,
        power_mixed_integer_with_context,
    },
    remainder_integer64::{
        remainder_integer, remainder_integer_with_context, remainder_mixed_integer,
        remainder_mixed_integer_with_context,
    },
    subtraction_integer64::{
        subtraction_integer, subtraction_integer_with_context, subtraction_mixed_integer,
        subtraction_mixed_integer_with_context,
    },
};

/// Registers every binary arithmetic operator for two `int64` operands.
///
/// Same-type operators keep `int64` as their static result while their
/// context-aware override promotes overflowed results to `int128` at runtime.
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

/// Registers every mixed-width operator whose static result is `int64`.
///
/// Mixed operators keep `int64` as their static result while their
/// context-aware override promotes overflowed results to `int128` at runtime.
pub(crate) fn register_promotions(
    registry: &mut Registry,
    narrower_ids: &[TypeId],
    integer64_id: TypeId,
) -> Result<(), CoreError> {
    for narrower_id in narrower_ids {
        for (operator, execute, execute_with_context) in [
            (
                BinaryOperator::Addition,
                addition_mixed_integer as language_core::BinaryOperatorExecutor,
                addition_mixed_integer_with_context as ContextBinaryOperatorExecutor,
            ),
            (
                BinaryOperator::Subtraction,
                subtraction_mixed_integer,
                subtraction_mixed_integer_with_context,
            ),
            (
                BinaryOperator::Multiplication,
                multiplication_mixed_integer,
                multiplication_mixed_integer_with_context,
            ),
            (
                BinaryOperator::Division,
                division_mixed_integer,
                division_mixed_integer_with_context,
            ),
            (
                BinaryOperator::Remainder,
                remainder_mixed_integer,
                remainder_mixed_integer_with_context,
            ),
            (
                BinaryOperator::Power,
                power_mixed_integer,
                power_mixed_integer_with_context,
            ),
        ] {
            register_mixed_pair(
                registry,
                operator,
                *narrower_id,
                integer64_id,
                execute,
                execute_with_context,
            )?;
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
    execute: language_core::BinaryOperatorExecutor,
    execute_with_context: ContextBinaryOperatorExecutor,
) -> Result<(), CoreError> {
    registry.register_context_binary_operator(
        operator,
        narrower_id,
        wider_id,
        wider_id,
        execute,
        execute_with_context,
    )?;
    registry.register_context_binary_operator(
        operator,
        wider_id,
        narrower_id,
        wider_id,
        execute,
        execute_with_context,
    )?;
    Ok(())
}
