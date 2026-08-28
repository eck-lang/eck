mod addition_double;
mod addition_float_double;
mod division_double;
mod division_float_double;
mod multiplication_double;
mod multiplication_float_double;
mod power_double;
mod power_float_double;
mod remainder_double;
mod remainder_float_double;
mod subtraction_double;
mod subtraction_float_double;

use language_core::{BinaryOperator, BinaryOperatorExecutor, CoreError, Registry, TypeId, Value};

use self::{
    addition_double::addition_double, addition_float_double::addition_float_double,
    division_double::division_double, division_float_double::division_float_double,
    multiplication_double::multiplication_double,
    multiplication_float_double::multiplication_float_double, power_double::power_double,
    power_float_double::power_float_double, remainder_double::remainder_double,
    remainder_float_double::remainder_float_double, subtraction_double::subtraction_double,
    subtraction_float_double::subtraction_float_double,
};

use crate::value::get as get_double;

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
        addition_float_double,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Subtraction,
        float_id,
        double_id,
        subtraction_float_double,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Multiplication,
        float_id,
        double_id,
        multiplication_float_double,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Division,
        float_id,
        double_id,
        division_float_double,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Remainder,
        float_id,
        double_id,
        remainder_float_double,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Power,
        float_id,
        double_id,
        power_float_double,
    )?;
    Ok(())
}

/// Registers one float/double operator in both source orders with a double result.
fn register_mixed(
    registry: &mut Registry,
    operator: BinaryOperator,
    float_id: TypeId,
    double_id: TypeId,
    execute: BinaryOperatorExecutor,
) -> Result<(), CoreError> {
    registry.register_binary_operator(operator, float_id, double_id, double_id, execute)?;
    registry.register_binary_operator(operator, double_id, float_id, double_id, execute)?;
    Ok(())
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
