use language_core::{BinaryOperator, CoreError, ExecutionContext, Value};

/// Extracts the signed 8-bit integer payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<i8, CoreError> {
    value
        .downcast_ref::<i8>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int8".into()))
}

/// Reports whether a failure is a checked fixed-width overflow.
///
/// Context-aware `int8` operators use this to decide between propagating the
/// original error (invalid representations, missing exponents) and promoting
/// the computation to `int16`. The message match mirrors the overflow errors
/// raised by the checked arithmetic in `operations/`; it is intentionally
/// narrow so only overflow takes the promotion path.
pub(crate) fn is_overflow_error(error: &CoreError) -> bool {
    matches!(error, CoreError::Runtime(message) if message.contains("overflow"))
}

/// Computes an overflowed `int8` operation with 16-bit precision as `int16`.
///
/// Callers pass the already extracted `i8` operands together with the
/// operator that overflowed. Division and remainder reject a zero divisor with
/// [`CoreError::DivisionByZero`] before any promotion, and power rejects an
/// exponent outside `u32` with the same error the checked implementation
/// reports. Promotion covers exactly one width step: when the result still
/// overflows `int16` (reachable only through power), the original overflow
/// error is preserved. Returns [`CoreError::Runtime`] when `int16` is not
/// registered in the execution registry.
pub(crate) fn promote_overflow_to_int16(
    context: &ExecutionContext<'_>,
    left_operand: i8,
    right_operand: i8,
    operator: BinaryOperator,
) -> Result<Value, CoreError> {
    let left_integer = i16::from(left_operand);
    let right_integer = i16::from(right_operand);
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
    let int16_id = context.registry().type_by_name("int16").ok_or_else(|| {
        CoreError::Runtime("int16 type is not registered for int8 overflow promotion".into())
    })?;
    Ok(Value::new(int16_id, promoted))
}

/// Returns the overflow error message for one arithmetic operator.
///
/// The messages match the checked implementations in `operations/` so a
/// residual `int16` overflow reports exactly the error the plain path would
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
