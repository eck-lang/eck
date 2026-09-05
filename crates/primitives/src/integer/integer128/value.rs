use language_core::{BinaryOperator, CoreError, ExecutionContext, Value};
use num_bigint::BigInt;

/// Extracts the signed 128-bit integer payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<i128, CoreError> {
    value
        .downcast_ref::<i128>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int128".into()))
}

/// Widens a mixed signed-integer pair to `int128` while preserving operand order.
///
/// Exactly one operand must use the `i128` representation and the other must use
/// a narrower signed fixed-width representation. The returned type identifier
/// belongs to the wider operand and is therefore the statically registered
/// result type.
pub(crate) fn mixed_operands(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<(i128, i128, language_core::TypeId), CoreError> {
    if let Some(left_integer) = left_operand.downcast_ref::<i128>() {
        if let Some(right_integer) = narrower_operand(right_operand) {
            return Ok((*left_integer, right_integer, left_operand.type_id()));
        }
    }
    if let Some(right_integer) = right_operand.downcast_ref::<i128>() {
        if let Some(left_integer) = narrower_operand(left_operand) {
            return Ok((left_integer, *right_integer, right_operand.type_id()));
        }
    }

    Err(CoreError::InvalidValueRepresentation(
        "narrower signed integer with int128 operands".into(),
    ))
}

/// Converts a signed integer narrower than `int128` without loss.
fn narrower_operand(value: &Value) -> Option<i128> {
    value
        .downcast_ref::<i8>()
        .map(|integer| i128::from(*integer))
        .or_else(|| {
            value
                .downcast_ref::<i16>()
                .map(|integer| i128::from(*integer))
        })
        .or_else(|| {
            value
                .downcast_ref::<i32>()
                .map(|integer| i128::from(*integer))
        })
        .or_else(|| {
            value
                .downcast_ref::<i64>()
                .map(|integer| i128::from(*integer))
        })
}

/// Reports whether a failure is a checked fixed-width overflow.
///
/// Context-aware `int128` operators use this to decide between propagating the
/// original error (invalid representations, missing exponents) and promoting
/// the computation to `bigint`. The message match mirrors the overflow errors
/// raised by the checked arithmetic in `operations/`; it is intentionally
/// narrow so only overflow takes the promotion path.
pub(crate) fn is_overflow_error(error: &CoreError) -> bool {
    matches!(error, CoreError::Runtime(message) if message.contains("overflow"))
}

/// Computes an overflowed `int128` operation with arbitrary precision as `bigint`.
///
/// Callers pass the already extracted `i128` operands together with the
/// operator that overflowed. Division and remainder reject a zero divisor with
/// [`CoreError::DivisionByZero`] before any promotion, and power rejects an
/// exponent outside `u32` with the same error the checked implementation
/// reports. Returns [`CoreError::Runtime`] when `bigint` is not registered in
/// the execution registry.
pub(crate) fn promote_overflow_to_bigint(
    context: &ExecutionContext<'_>,
    left_operand: i128,
    right_operand: i128,
    operator: BinaryOperator,
) -> Result<Value, CoreError> {
    let left_integer = BigInt::from(left_operand);
    let right_integer = BigInt::from(right_operand);
    let promoted = match operator {
        BinaryOperator::Addition => left_integer + right_integer,
        BinaryOperator::Subtraction => left_integer - right_integer,
        BinaryOperator::Multiplication => left_integer * right_integer,
        BinaryOperator::Division => {
            if right_operand == 0 {
                return Err(CoreError::DivisionByZero);
            }
            left_integer / right_integer
        }
        BinaryOperator::Remainder => {
            if right_operand == 0 {
                return Err(CoreError::DivisionByZero);
            }
            left_integer % right_integer
        }
        BinaryOperator::Power => {
            let exponent = u32::try_from(right_operand).map_err(|_| {
                CoreError::Runtime(
                    "integer power exponent must be non-negative and fit in u32".into(),
                )
            })?;
            left_integer.pow(exponent)
        }
    };
    let bigint_id = context.registry().type_by_name("bigint").ok_or_else(|| {
        CoreError::Runtime("bigint type is not registered for int128 overflow promotion".into())
    })?;
    Ok(Value::new(bigint_id, promoted))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
