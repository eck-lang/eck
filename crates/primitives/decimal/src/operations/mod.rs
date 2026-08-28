mod addition;
mod division;
mod multiplication;
mod power;
mod remainder;
mod subtraction;

pub(crate) use self::addition::{
    addition_decimal, addition_decimal_double, addition_decimal_float, addition_decimal_int,
};
pub(crate) use self::division::{
    division_decimal, division_decimal_double, division_decimal_float, division_decimal_int,
};
pub(crate) use self::multiplication::{
    multiplication_decimal, multiplication_decimal_double, multiplication_decimal_float,
    multiplication_decimal_int,
};
pub(crate) use self::power::{
    power_decimal, power_decimal_double, power_decimal_float, power_decimal_int,
};
pub(crate) use self::remainder::{
    remainder_decimal, remainder_decimal_double, remainder_decimal_float, remainder_decimal_int,
};
pub(crate) use self::subtraction::{
    subtraction_decimal, subtraction_decimal_double, subtraction_decimal_float,
    subtraction_decimal_int,
};

use language_core::{BinaryOperator, BinaryOperatorExecutor, CoreError, Registry, TypeId, Value};
use rust_decimal::{Decimal, prelude::ToPrimitive};

use crate::value::get as get_decimal;

/// Registers decimal operators and their mixed-type overloads.
pub(crate) fn register(registry: &mut Registry, decimal_id: TypeId) -> Result<(), CoreError> {
    registry.register_binary_operator(
        BinaryOperator::Addition,
        decimal_id,
        decimal_id,
        decimal_id,
        addition_decimal,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Subtraction,
        decimal_id,
        decimal_id,
        decimal_id,
        subtraction_decimal,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Multiplication,
        decimal_id,
        decimal_id,
        decimal_id,
        multiplication_decimal,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Division,
        decimal_id,
        decimal_id,
        decimal_id,
        division_decimal,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Remainder,
        decimal_id,
        decimal_id,
        decimal_id,
        remainder_decimal,
    )?;
    registry.register_binary_operator(
        BinaryOperator::Power,
        decimal_id,
        decimal_id,
        decimal_id,
        power_decimal,
    )?;

    if let Some(float_id) = registry.type_by_name("float") {
        register_mixed_float(registry, decimal_id, float_id)?;
    }
    if let Some(double_id) = registry.type_by_name("double") {
        register_mixed_double(registry, decimal_id, double_id)?;
    }
    if let Some(integer_id) = registry.type_by_name("int") {
        register_mixed_int(registry, decimal_id, integer_id)?;
    }

    Ok(())
}

/// Registers all decimal/float overloads using one callback per operation.
fn register_mixed_float(
    registry: &mut Registry,
    decimal_id: TypeId,
    float_id: TypeId,
) -> Result<(), CoreError> {
    register_mixed(
        registry,
        BinaryOperator::Addition,
        decimal_id,
        float_id,
        addition_decimal_float,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Subtraction,
        decimal_id,
        float_id,
        subtraction_decimal_float,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Multiplication,
        decimal_id,
        float_id,
        multiplication_decimal_float,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Division,
        decimal_id,
        float_id,
        division_decimal_float,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Remainder,
        decimal_id,
        float_id,
        remainder_decimal_float,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Power,
        decimal_id,
        float_id,
        power_decimal_float,
    )?;
    Ok(())
}

/// Registers all decimal/double overloads using one callback per operation.
fn register_mixed_double(
    registry: &mut Registry,
    decimal_id: TypeId,
    double_id: TypeId,
) -> Result<(), CoreError> {
    register_mixed(
        registry,
        BinaryOperator::Addition,
        decimal_id,
        double_id,
        addition_decimal_double,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Subtraction,
        decimal_id,
        double_id,
        subtraction_decimal_double,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Multiplication,
        decimal_id,
        double_id,
        multiplication_decimal_double,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Division,
        decimal_id,
        double_id,
        division_decimal_double,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Remainder,
        decimal_id,
        double_id,
        remainder_decimal_double,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Power,
        decimal_id,
        double_id,
        power_decimal_double,
    )?;
    Ok(())
}

/// Registers all decimal/int overloads using one callback per operation.
fn register_mixed_int(
    registry: &mut Registry,
    decimal_id: TypeId,
    integer_id: TypeId,
) -> Result<(), CoreError> {
    register_mixed(
        registry,
        BinaryOperator::Addition,
        decimal_id,
        integer_id,
        addition_decimal_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Subtraction,
        decimal_id,
        integer_id,
        subtraction_decimal_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Multiplication,
        decimal_id,
        integer_id,
        multiplication_decimal_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Division,
        decimal_id,
        integer_id,
        division_decimal_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Remainder,
        decimal_id,
        integer_id,
        remainder_decimal_int,
    )?;
    register_mixed(
        registry,
        BinaryOperator::Power,
        decimal_id,
        integer_id,
        power_decimal_int,
    )?;
    Ok(())
}

/// Registers one mixed operator in both operand orders.
fn register_mixed(
    registry: &mut Registry,
    operator: BinaryOperator,
    decimal_id: TypeId,
    other_id: TypeId,
    execute: BinaryOperatorExecutor,
) -> Result<(), CoreError> {
    registry.register_binary_operator(operator, decimal_id, other_id, decimal_id, execute)?;
    registry.register_binary_operator(operator, other_id, decimal_id, decimal_id, execute)?;
    Ok(())
}

/// Converts a decimal/float pair into decimal operands in source order.
pub(super) fn decimal_float_operands(
    lhs: &Value,
    rhs: &Value,
) -> Result<(Decimal, Decimal, TypeId), CoreError> {
    if let Some(decimal_lhs) = lhs.downcast_ref::<Decimal>() {
        let float_rhs = get_float(rhs)?;
        return Ok((*decimal_lhs, float_as_decimal(float_rhs)?, lhs.type_id()));
    }

    if let Some(float_lhs) = lhs.downcast_ref::<f32>() {
        let decimal_rhs = get_decimal(rhs)?;
        return Ok((float_as_decimal(*float_lhs)?, decimal_rhs, rhs.type_id()));
    }

    Err(CoreError::InvalidValueRepresentation(
        "decimal or float".into(),
    ))
}

/// Converts a decimal/double pair into decimal operands in source order.
pub(super) fn decimal_double_operands(
    lhs: &Value,
    rhs: &Value,
) -> Result<(Decimal, Decimal, TypeId), CoreError> {
    if let Some(decimal_lhs) = lhs.downcast_ref::<Decimal>() {
        let double_rhs = get_double(rhs)?;
        return Ok((*decimal_lhs, double_as_decimal(double_rhs)?, lhs.type_id()));
    }

    if let Some(double_lhs) = lhs.downcast_ref::<f64>() {
        let decimal_rhs = get_decimal(rhs)?;
        return Ok((double_as_decimal(*double_lhs)?, decimal_rhs, rhs.type_id()));
    }

    Err(CoreError::InvalidValueRepresentation(
        "decimal or double".into(),
    ))
}

/// Converts a decimal/int pair into decimal operands in source order.
pub(super) fn decimal_int_operands(
    lhs: &Value,
    rhs: &Value,
) -> Result<(Decimal, Decimal, TypeId), CoreError> {
    if let Some(decimal_lhs) = lhs.downcast_ref::<Decimal>() {
        let integer_rhs = get_integer(rhs)?;
        return Ok((*decimal_lhs, Decimal::from(integer_rhs), lhs.type_id()));
    }

    if let Some(integer_lhs) = lhs.downcast_ref::<i64>() {
        let decimal_rhs = get_decimal(rhs)?;
        return Ok((Decimal::from(*integer_lhs), decimal_rhs, rhs.type_id()));
    }

    Err(CoreError::InvalidValueRepresentation(
        "decimal or int".into(),
    ))
}

/// Extracts an `f32` from a runtime value.
fn get_float(value: &Value) -> Result<f32, CoreError> {
    value
        .downcast_ref::<f32>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("float".into()))
}

/// Extracts an `f64` from a runtime value.
fn get_double(value: &Value) -> Result<f64, CoreError> {
    value
        .downcast_ref::<f64>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("double".into()))
}

/// Extracts an `i64` from a runtime value.
fn get_integer(value: &Value) -> Result<i64, CoreError> {
    value
        .downcast_ref::<i64>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".into()))
}

/// Converts a finite floating-point value to a decimal value.
fn float_as_decimal(value: f32) -> Result<Decimal, CoreError> {
    Decimal::try_from(value)
        .map_err(|error| CoreError::Runtime(format!("cannot convert float to decimal: {error}")))
}

/// Converts a finite double-precision value to a decimal value.
fn double_as_decimal(value: f64) -> Result<Decimal, CoreError> {
    Decimal::try_from(value)
        .map_err(|error| CoreError::Runtime(format!("cannot convert double to decimal: {error}")))
}

/// Converts an integer-valued decimal exponent into an `i64`.
pub(super) fn decimal_exponent(value: Decimal) -> Result<i64, CoreError> {
    if !value.is_integer() {
        return Err(CoreError::Runtime(
            "decimal power exponent must be an integer".into(),
        ));
    }
    value
        .to_i64()
        .ok_or_else(|| CoreError::Runtime("decimal power exponent does not fit in i64".into()))
}

/// Adds decimals and converts overflow into a language error.
pub(super) fn checked_addition(lhs: Decimal, rhs: Decimal) -> Result<Decimal, CoreError> {
    lhs.checked_add(rhs)
        .ok_or_else(|| CoreError::Runtime("decimal overflow in addition".into()))
}

/// Subtracts decimals and converts overflow into a language error.
pub(super) fn checked_subtraction(lhs: Decimal, rhs: Decimal) -> Result<Decimal, CoreError> {
    lhs.checked_sub(rhs)
        .ok_or_else(|| CoreError::Runtime("decimal overflow in subtraction".into()))
}

/// Multiplies decimals and converts overflow into a language error.
pub(super) fn checked_multiplication(lhs: Decimal, rhs: Decimal) -> Result<Decimal, CoreError> {
    lhs.checked_mul(rhs)
        .ok_or_else(|| CoreError::Runtime("decimal overflow in multiplication".into()))
}

/// Divides decimals and converts arithmetic overflow into a language error.
pub(super) fn checked_division(lhs: Decimal, rhs: Decimal) -> Result<Decimal, CoreError> {
    lhs.checked_div(rhs)
        .ok_or_else(|| CoreError::Runtime("decimal overflow in division".into()))
}

/// Calculates a decimal remainder and converts failure into a language error.
pub(super) fn checked_remainder(lhs: Decimal, rhs: Decimal) -> Result<Decimal, CoreError> {
    lhs.checked_rem(rhs)
        .ok_or_else(|| CoreError::Runtime("decimal overflow in remainder".into()))
}

/// Raises a decimal to an integer exponent using checked multiplication.
pub(super) fn checked_power(base: Decimal, exponent: i64) -> Result<Decimal, CoreError> {
    let negative = exponent < 0;
    let mut remaining = exponent.unsigned_abs();
    let mut factor = base;
    let mut result = Decimal::ONE;

    while remaining > 0 {
        if remaining & 1 == 1 {
            result = result
                .checked_mul(factor)
                .ok_or_else(|| CoreError::Runtime("decimal overflow in power".into()))?;
        }
        remaining >>= 1;
        if remaining > 0 {
            factor = factor
                .checked_mul(factor)
                .ok_or_else(|| CoreError::Runtime("decimal overflow in power".into()))?;
        }
    }

    if negative {
        if result.is_zero() {
            return Err(CoreError::DivisionByZero);
        }
        Decimal::ONE
            .checked_div(result)
            .ok_or_else(|| CoreError::Runtime("decimal overflow in power".into()))
    } else {
        Ok(result)
    }
}
