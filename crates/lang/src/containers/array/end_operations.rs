//! Built-in operations at either end of an array payload, plus element access.
//!
//! These operations are language intrinsics rather than registered functions:
//! the compiler resolves a source spelling to one [`ArrayEndOperation`] and the runtime
//! applies it here. The payload owns the element movement, the copy-on-write
//! decision, and the bounds contract, so the runtime keeps only expression
//! evaluation and local-slot flow.

use crate::semantic::{ArrayEndOperation, ArrayType, CoreError, Value};

use super::value::{ArrayValue, invalid_array_value};

/// Applies one end operation to the array payload stored in `value`.
///
/// An adding operation stores the value it was given and reports nothing, while
/// a removing operation reports the element it moved out of the array. A removal
/// from an empty array reports nothing and leaves the array unchanged, which lets
/// the caller produce the language's null value.
///
/// `stored` must be `Some` exactly when `method` adds an element, which the
/// compiled program guarantees.
pub fn apply_end_operation(
    value: &mut Value,
    method: ArrayEndOperation,
    stored: Option<Value>,
) -> Result<Option<Value>, CoreError> {
    let array_type = value.array_type().ok_or_else(invalid_array_value)?;
    if method.removes_element() && ArrayValue::from_value(value)?.length() == 0 {
        return Ok(None);
    }
    copy_if_shared(value, array_type)?;
    let array = ArrayValue::from_value_mut(value)?;
    match method {
        ArrayEndOperation::Push => {
            array.push(stored.expect("an adding operation has one value to store"));
            Ok(None)
        }
        ArrayEndOperation::Unshift => {
            array.unshift(stored.expect("an adding operation has one value to store"));
            Ok(None)
        }
        ArrayEndOperation::Pop => Ok(array.pop()),
        ArrayEndOperation::Shift => Ok(array.shift()),
    }
}

/// Returns a copy of the element at `index`, or the stable bounds error.
pub fn element_at(value: &Value, index: usize) -> Result<Value, CoreError> {
    let array = ArrayValue::from_value(value)?;
    let length = array.length();
    array
        .elements()
        .get(index)
        .cloned()
        .ok_or(CoreError::ArrayIndexOutOfBounds { index, length })
}

/// Replaces the element at `index`, copying the payload when it is shared.
///
/// The bounds are checked before the copy-on-write decision, so a rejected index
/// leaves a shared payload aliased instead of duplicating it for nothing.
pub fn set_element(value: &mut Value, index: usize, element: Value) -> Result<(), CoreError> {
    let array_type = value.array_type().ok_or_else(invalid_array_value)?;
    let length = ArrayValue::from_value(value)?.length();
    if index >= length {
        return Err(CoreError::ArrayIndexOutOfBounds { index, length });
    }
    copy_if_shared(value, array_type)?;
    ArrayValue::from_value_mut(value)?.elements_mut()[index] = element;
    Ok(())
}

/// Gives a shared payload its own copy so a mutation stays invisible to aliases.
///
/// An array has value semantics: while another binding observes the same
/// allocation, a mutation must not alias into it. The copy happens only when the
/// payload is shared, which keeps the ordinary uniquely-owned path allocation
/// free.
fn copy_if_shared(value: &mut Value, array_type: ArrayType) -> Result<(), CoreError> {
    if value.is_uniquely_owned() {
        return Ok(());
    }
    let copied = ArrayValue::from_value(value)?.clone();
    *value = Value::new_array(array_type, copied);
    Ok(())
}

#[cfg(test)]
#[path = "end_operations.tests.rs"]
mod tests;
