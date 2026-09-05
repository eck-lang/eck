use language_core::{CoreError, Value};

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

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
