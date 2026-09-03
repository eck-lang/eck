use language_core::{CoreError, Value};

/// Extracts the signed 16-bit integer payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<i16, CoreError> {
    value
        .downcast_ref::<i16>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int16".into()))
}

/// Widens a mixed signed-integer pair to `int16` while preserving operand order.
///
/// Exactly one operand must use the `i16` representation and the other must use
/// an `i8` representation. The returned type identifier belongs to the wider
/// operand and is therefore the statically registered result type.
pub(crate) fn mixed_operands(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<(i16, i16, language_core::TypeId), CoreError> {
    if let Some(left_integer) = left_operand.downcast_ref::<i16>() {
        if let Some(right_integer) = right_operand.downcast_ref::<i8>() {
            return Ok((
                *left_integer,
                i16::from(*right_integer),
                left_operand.type_id(),
            ));
        }
    }
    if let Some(right_integer) = right_operand.downcast_ref::<i16>() {
        if let Some(left_integer) = left_operand.downcast_ref::<i8>() {
            return Ok((
                i16::from(*left_integer),
                *right_integer,
                right_operand.type_id(),
            ));
        }
    }

    Err(CoreError::InvalidValueRepresentation(
        "int8 and int16 operands".into(),
    ))
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
