use language_core::{CoreError, Value};

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

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
