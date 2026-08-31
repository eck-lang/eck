mod addition;
mod division;
mod multiplication;
mod power;
mod remainder;
mod subtraction;

use language_core::{BinaryOperator, BinaryOperatorExecutor, CoreError, Registry, TypeId, Value};

use self::{
    addition::{addition_double, addition_double_float, addition_double_int},
    division::{division_double, division_double_float, division_double_int},
    multiplication::{
        multiplication_double, multiplication_double_float, multiplication_double_int,
    },
    power::{power_double, power_double_float, power_double_int},
    remainder::{remainder_double, remainder_double_float, remainder_double_int},
    subtraction::{subtraction_double, subtraction_double_float, subtraction_double_int},
};

use crate::double::value::get as get_double;

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

    if let Some(float_id) = registry.type_by_name("float") {
        register_mixed_float(registry, float_id, double_id)?;
    }
    if let Some(integer_id) = registry.type_by_name("int") {
        register_mixed_int(registry, integer_id, double_id)?;
    }
    Ok(())
}

/// Registers every operator that promotes a `float` operand to `double`.
fn register_mixed_float(
    registry: &mut Registry,
    float_id: TypeId,
    double_id: TypeId,
) -> Result<(), CoreError> {
    register_mixed(
        registry,
        BinaryOperator::Addition,
        float_id,
        double_id,
        addition_double_float,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Subtraction,
        float_id,
        double_id,
        subtraction_double_float,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Multiplication,
        float_id,
        double_id,
        multiplication_double_float,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Division,
        float_id,
        double_id,
        division_double_float,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Remainder,
        float_id,
        double_id,
        remainder_double_float,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Power,
        float_id,
        double_id,
        power_double_float,
    )?;
    Ok(())
}

/// Registers every operator that promotes an `int` operand to `double`.
fn register_mixed_int(
    registry: &mut Registry,
    integer_id: TypeId,
    double_id: TypeId,
) -> Result<(), CoreError> {
    register_mixed(
        registry,
        BinaryOperator::Addition,
        integer_id,
        double_id,
        addition_double_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Subtraction,
        integer_id,
        double_id,
        subtraction_double_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Multiplication,
        integer_id,
        double_id,
        multiplication_double_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Division,
        integer_id,
        double_id,
        division_double_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Remainder,
        integer_id,
        double_id,
        remainder_double_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Power,
        integer_id,
        double_id,
        power_double_int,
    )?;
    Ok(())
}

/// Registers one mixed operator in both source orders with a double result.
fn register_mixed(
    registry: &mut Registry,
    operator: BinaryOperator,
    other_id: TypeId,
    double_id: TypeId,
    execute: BinaryOperatorExecutor,
) -> Result<(), CoreError> {
    registry.register_binary_operator(operator, other_id, double_id, double_id, execute)?;
    registry.register_binary_operator(operator, double_id, other_id, double_id, execute)?;
    Ok(())
}

/// Converts a double/int pair to double operands while preserving source order.
///
/// Conversion to `f64` follows the result representation and may round integer
/// magnitudes that are not exactly representable in double precision.
pub(super) fn double_int_operands(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<(f64, f64, TypeId), CoreError> {
    if let Some(double_left_operand) = left_operand.downcast_ref::<f64>() {
        let integer_right_operand = get_integer(right_operand)?;
        return Ok((
            *double_left_operand,
            integer_right_operand as f64,
            left_operand.type_id(),
        ));
    }

    if let Some(integer_left_operand) = left_operand.downcast_ref::<i64>() {
        let double_right_operand = get_double(right_operand)?;
        return Ok((
            *integer_left_operand as f64,
            double_right_operand,
            right_operand.type_id(),
        ));
    }

    Err(CoreError::InvalidValueRepresentation(
        "double or int".into(),
    ))
}

/// Converts a float/double pair to double operands while preserving source order.
pub(super) fn float_double_operands(
    lhs: &Value,
    rhs: &Value,
) -> Result<(f64, f64, TypeId), CoreError> {
    if let Some(float_lhs) = lhs.downcast_ref::<f32>() {
        return Ok((f64::from(*float_lhs), get_double(rhs)?, rhs.type_id()));
    }

    if let Some(double_lhs) = lhs.downcast_ref::<f64>() {
        return Ok((*double_lhs, f64::from(get_float(rhs)?), lhs.type_id()));
    }

    Err(CoreError::InvalidValueRepresentation(
        "float or double".into(),
    ))
}

/// Extracts the single-precision floating-point payload from a runtime value.
fn get_float(value: &Value) -> Result<f32, CoreError> {
    value
        .downcast_ref::<f32>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("float".into()))
}

/// Extracts the signed 64-bit integer payload from a runtime value.
fn get_integer(value: &Value) -> Result<i64, CoreError> {
    value
        .downcast_ref::<i64>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".into()))
}
