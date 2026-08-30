mod addition;
mod division;
mod multiplication;
mod power;
mod remainder;
mod subtraction;

use language_core::{BinaryOperator, BinaryOperatorExecutor, CoreError, Registry, TypeId, Value};

use self::{
    addition::{addition_float, addition_float_int},
    division::{division_float, division_float_int},
    multiplication::{multiplication_float, multiplication_float_int},
    power::{power_float, power_float_int},
    remainder::{remainder_float, remainder_float_int},
    subtraction::{subtraction_float, subtraction_float_int},
};

use crate::value::get as get_float;

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

    if let Some(integer_id) = registry.type_by_name("int") {
        register_mixed_int(registry, integer_id, float_id)?;
    }
    Ok(())
}

/// Registers every operator that promotes an `int` operand to `float`.
fn register_mixed_int(
    registry: &mut Registry,
    integer_id: TypeId,
    float_id: TypeId,
) -> Result<(), CoreError> {
    register_mixed(
        registry,
        BinaryOperator::Addition,
        integer_id,
        float_id,
        addition_float_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Subtraction,
        integer_id,
        float_id,
        subtraction_float_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Multiplication,
        integer_id,
        float_id,
        multiplication_float_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Division,
        integer_id,
        float_id,
        division_float_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Remainder,
        integer_id,
        float_id,
        remainder_float_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Power,
        integer_id,
        float_id,
        power_float_int,
    )?;
    Ok(())
}

/// Registers one mixed operator in both source orders with a float result.
fn register_mixed(
    registry: &mut Registry,
    operator: BinaryOperator,
    integer_id: TypeId,
    float_id: TypeId,
    execute: BinaryOperatorExecutor,
) -> Result<(), CoreError> {
    registry.register_binary_operator(operator, integer_id, float_id, float_id, execute)?;
    registry.register_binary_operator(operator, float_id, integer_id, float_id, execute)?;
    Ok(())
}

/// Converts a float/int pair to float operands while preserving source order.
///
/// Conversion to `f32` follows the result representation and may round integer
/// magnitudes that are not exactly representable in single precision.
pub(super) fn float_int_operands(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<(f32, f32, TypeId), CoreError> {
    if let Some(float_left_operand) = left_operand.downcast_ref::<f32>() {
        let integer_right_operand = get_integer(right_operand)?;
        return Ok((
            *float_left_operand,
            integer_right_operand as f32,
            left_operand.type_id(),
        ));
    }

    if let Some(integer_left_operand) = left_operand.downcast_ref::<i64>() {
        let float_right_operand = get_float(right_operand)?;
        return Ok((
            *integer_left_operand as f32,
            float_right_operand,
            right_operand.type_id(),
        ));
    }

    Err(CoreError::InvalidValueRepresentation("float or int".into()))
}

/// Extracts the signed 64-bit integer payload from a runtime value.
fn get_integer(value: &Value) -> Result<i64, CoreError> {
    value
        .downcast_ref::<i64>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".into()))
}
