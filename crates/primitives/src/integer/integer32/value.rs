use language_core::{BinaryOperator, CoreError, ExecutionContext, Value};

/// Extracts the signed 32-bit integer payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<i32, CoreError> {
    value
        .downcast_ref::<i32>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int32".into()))
}

/// Widens a mixed signed-integer pair to `int32` while preserving operand order.
///
/// Exactly one operand must use the `i32` representation and the other must use
/// an `i8` or `i16` representation. The returned type identifier belongs to the
/// wider operand and is therefore the statically registered result type.
pub(crate) fn mixed_operands(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<(i32, i32, language_core::TypeId), CoreError> {
    if let Some(left_integer) = left_operand.downcast_ref::<i32>() {
        if let Some(right_integer) = narrower_operand(right_operand) {
            return Ok((*left_integer, right_integer, left_operand.type_id()));
        }
    }
    if let Some(right_integer) = right_operand.downcast_ref::<i32>() {
        if let Some(left_integer) = narrower_operand(left_operand) {
            return Ok((left_integer, *right_integer, right_operand.type_id()));
        }
    }

    Err(CoreError::InvalidValueRepresentation(
        "int8 or int16 with int32 operands".into(),
    ))
}

/// Converts a signed integer narrower than `int32` without loss.
fn narrower_operand(value: &Value) -> Option<i32> {
    value
        .downcast_ref::<i8>()
        .map(|integer| i32::from(*integer))
        .or_else(|| {
            value
                .downcast_ref::<i16>()
                .map(|integer| i32::from(*integer))
        })
}

/// Reports whether a failure is a checked fixed-width overflow.
///
/// Context-aware `int32` operators use this to decide between propagating the
/// original error (invalid representations, missing exponents) and promoting
/// the computation to `int64`. The message match mirrors the overflow errors
/// raised by the checked arithmetic in `operations/`; it is intentionally
/// narrow so only overflow takes the promotion path.
pub(crate) fn is_overflow_error(error: &CoreError) -> bool {
    matches!(error, CoreError::Runtime(message) if message.contains("overflow"))
}

/// Computes an overflowed `int32` operation with 64-bit precision as `int64`.
///
/// Callers pass the already extracted `i32` operands together with the
/// operator that overflowed. Division and remainder reject a zero divisor with
/// [`CoreError::DivisionByZero`] before any promotion, and power rejects an
/// exponent outside `u32` with the same error the checked implementation
/// reports. Promotion covers exactly one width step: when the result still
/// overflows `int64` (reachable through power), the original overflow error is
/// preserved. Returns [`CoreError::Runtime`] when `int64` is not registered in
/// the execution registry.
pub(crate) fn promote_overflow_to_int64(
    context: &ExecutionContext<'_>,
    left_operand: i32,
    right_operand: i32,
    operator: BinaryOperator,
) -> Result<Value, CoreError> {
    let left_integer = i64::from(left_operand);
    let right_integer = i64::from(right_operand);
    let promoted = match operator {
        BinaryOperator::Addition => left_integer.checked_add(right_integer),
        BinaryOperator::Subtraction => left_integer.checked_sub(right_integer),
        BinaryOperator::Multiplication => left_integer.checked_mul(right_integer),
        BinaryOperator::Division => {
            if right_operand == 0 {
                return Err(CoreError::DivisionByZero);
            }
            left_integer.checked_div(right_integer)
        }
        BinaryOperator::Remainder => {
            if right_operand == 0 {
                return Err(CoreError::DivisionByZero);
            }
            left_integer.checked_rem(right_integer)
        }
        BinaryOperator::Power => {
            let exponent = u32::try_from(right_operand).map_err(|_| {
                CoreError::Runtime(
                    "integer power exponent must be non-negative and fit in u32".into(),
                )
            })?;
            left_integer.checked_pow(exponent)
        }
    };
    let promoted = promoted.ok_or_else(|| CoreError::Runtime(overflow_message(operator).into()))?;
    let int64_id = context.registry().type_by_name("int64").ok_or_else(|| {
        CoreError::Runtime("int64 type is not registered for int32 overflow promotion".into())
    })?;
    Ok(Value::new(int64_id, promoted))
}

/// Returns the overflow error message for one arithmetic operator.
///
/// The messages match the checked implementations in `operations/` so a
/// residual `int64` overflow reports exactly the error the plain path would
/// have reported.
fn overflow_message(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Addition => "integer overflow in addition",
        BinaryOperator::Subtraction => "integer overflow in subtraction",
        BinaryOperator::Multiplication => "integer overflow in multiplication",
        BinaryOperator::Division => "integer overflow in division",
        BinaryOperator::Remainder => "integer overflow in remainder",
        BinaryOperator::Power => "integer overflow in power",
    }
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
