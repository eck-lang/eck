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

use crate::decimal::value::{from_double, from_float, get as get_decimal};

/// Registers decimal arithmetic operators and their mixed-type overloads.
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
    left_operand: &Value,
    right_operand: &Value,
) -> Result<(Decimal, Decimal, TypeId), CoreError> {
    if let Some(decimal_left_operand) = left_operand.downcast_ref::<Decimal>() {
        let float_right_operand = get_float(right_operand)?;
        return Ok((
            *decimal_left_operand,
            from_float(float_right_operand)?,
            left_operand.type_id(),
        ));
    }

    if let Some(float_left_operand) = left_operand.downcast_ref::<f32>() {
        let decimal_right_operand = get_decimal(right_operand)?;
        return Ok((
            from_float(*float_left_operand)?,
            decimal_right_operand,
            right_operand.type_id(),
        ));
    }

    Err(CoreError::InvalidValueRepresentation(
        "decimal or float".into(),
    ))
}

/// Converts a decimal/double pair into decimal operands in source order.
pub(super) fn decimal_double_operands(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<(Decimal, Decimal, TypeId), CoreError> {
    if let Some(decimal_left_operand) = left_operand.downcast_ref::<Decimal>() {
        let double_right_operand = get_double(right_operand)?;
        return Ok((
            *decimal_left_operand,
            from_double(double_right_operand)?,
            left_operand.type_id(),
        ));
    }

    if let Some(double_left_operand) = left_operand.downcast_ref::<f64>() {
        let decimal_right_operand = get_decimal(right_operand)?;
        return Ok((
            from_double(*double_left_operand)?,
            decimal_right_operand,
            right_operand.type_id(),
        ));
    }

    Err(CoreError::InvalidValueRepresentation(
        "decimal or double".into(),
    ))
}

/// Converts a decimal/int pair into decimal operands in source order.
pub(super) fn decimal_int_operands(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<(Decimal, Decimal, TypeId), CoreError> {
    if let Some(decimal_left_operand) = left_operand.downcast_ref::<Decimal>() {
        let integer_right_operand = get_integer(right_operand)?;
        return Ok((
            *decimal_left_operand,
            Decimal::from(integer_right_operand),
            left_operand.type_id(),
        ));
    }

    if let Some(integer_left_operand) = left_operand.downcast_ref::<i64>() {
        let decimal_right_operand = get_decimal(right_operand)?;
        return Ok((
            Decimal::from(*integer_left_operand),
            decimal_right_operand,
            right_operand.type_id(),
        ));
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
pub(super) fn checked_addition(
    left_operand: Decimal,
    right_operand: Decimal,
) -> Result<Decimal, CoreError> {
    left_operand
        .checked_add(right_operand)
        .ok_or_else(|| CoreError::Runtime("decimal overflow in addition".into()))
}

/// Subtracts decimals and converts overflow into a language error.
pub(super) fn checked_subtraction(
    left_operand: Decimal,
    right_operand: Decimal,
) -> Result<Decimal, CoreError> {
    left_operand
        .checked_sub(right_operand)
        .ok_or_else(|| CoreError::Runtime("decimal overflow in subtraction".into()))
}

/// Multiplies decimals and converts overflow into a language error.
pub(super) fn checked_multiplication(
    left_operand: Decimal,
    right_operand: Decimal,
) -> Result<Decimal, CoreError> {
    left_operand
        .checked_mul(right_operand)
        .ok_or_else(|| CoreError::Runtime("decimal overflow in multiplication".into()))
}

/// Divides decimals and converts arithmetic overflow into a language error.
pub(super) fn checked_division(
    left_operand: Decimal,
    right_operand: Decimal,
) -> Result<Decimal, CoreError> {
    left_operand
        .checked_div(right_operand)
        .ok_or_else(|| CoreError::Runtime("decimal overflow in division".into()))
}

/// Calculates a decimal remainder and converts failure into a language error.
pub(super) fn checked_remainder(
    left_operand: Decimal,
    right_operand: Decimal,
) -> Result<Decimal, CoreError> {
    left_operand
        .checked_rem(right_operand)
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
