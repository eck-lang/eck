use language_core::{CoreError, Value};
use num_bigint::BigInt;

/// Extracts the arbitrary-precision integer payload from a runtime value.
pub(crate) fn get(value: &Value) -> Result<&BigInt, CoreError> {
    value
        .downcast_ref::<BigInt>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("bigint".into()))
}

/// Converts this arbitrary-precision value into a zero-based array index.
///
/// The decimal spelling is re-parsed as a machine-word index, which rejects a
/// negative or oversized magnitude through the `usize` parser. An index larger
/// than the machine word size can never address a real array, so formatting
/// then parsing keeps the check exact without truncating the index.
pub(crate) fn to_index(value: &Value) -> Result<usize, CoreError> {
    let integer = get(value)?;
    integer.to_string().parse::<usize>().map_err(|_| {
        CoreError::Runtime(format!(
            "array index {integer} must be a non-negative integer that fits in a machine word"
        ))
    })
}

/// Extracts a uniquely owned arbitrary-precision integer payload for mutation.
pub(crate) fn get_mut(value: &mut Value) -> Result<&mut BigInt, CoreError> {
    value
        .downcast_mut::<BigInt>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("uniquely owned bigint".into()))
}

/// Widens a mixed signed-integer pair to `bigint` while preserving operand order.
///
/// Exactly one operand must use the `BigInt` representation and the other must
/// use a narrower signed fixed-width representation. The returned type
/// identifier belongs to the wider operand and is therefore the statically
/// registered result type.
pub(crate) fn mixed_operands(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<(BigInt, BigInt, language_core::TypeId), CoreError> {
    if let Some(left_integer) = left_operand.downcast_ref::<BigInt>() {
        if let Some(right_integer) = narrower_operand(right_operand) {
            return Ok((left_integer.clone(), right_integer, left_operand.type_id()));
        }
    }
    if let Some(right_integer) = right_operand.downcast_ref::<BigInt>() {
        if let Some(left_integer) = narrower_operand(left_operand) {
            return Ok((left_integer, right_integer.clone(), right_operand.type_id()));
        }
    }

    Err(CoreError::InvalidValueRepresentation(
        "narrower signed integer with bigint operands".into(),
    ))
}

/// Converts a signed integer narrower than `bigint` without loss.
fn narrower_operand(value: &Value) -> Option<BigInt> {
    value
        .downcast_ref::<i8>()
        .map(|integer| BigInt::from(*integer))
        .or_else(|| {
            value
                .downcast_ref::<i16>()
                .map(|integer| BigInt::from(*integer))
        })
        .or_else(|| {
            value
                .downcast_ref::<i32>()
                .map(|integer| BigInt::from(*integer))
        })
        .or_else(|| {
            value
                .downcast_ref::<i64>()
                .map(|integer| BigInt::from(*integer))
        })
        .or_else(|| {
            value
                .downcast_ref::<i128>()
                .map(|integer| BigInt::from(*integer))
        })
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
