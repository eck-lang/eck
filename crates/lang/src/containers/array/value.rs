use crate::semantic::{CoreError, Value};

use crate::containers::array::storage::ArrayStorage;

/// Public runtime payload for an ECK array.
pub struct ArrayValue {
    /// Private contiguous storage for the initialized array elements.
    storage: ArrayStorage,
}

impl ArrayValue {
    /// Creates an array from values already prepared for its element contract.
    pub fn new(elements: Vec<Value>) -> Self {
        Self {
            storage: ArrayStorage::new(elements),
        }
    }

    /// Returns the number of live elements.
    #[inline]
    pub fn length(&self) -> usize {
        self.storage.length()
    }

    /// Borrows all live elements as one contiguous slice.
    #[inline]
    pub fn elements(&self) -> &[Value] {
        self.storage.elements()
    }

    /// Mutably borrows all live elements.
    #[inline]
    pub fn elements_mut(&mut self) -> &mut [Value] {
        self.storage.elements_mut()
    }

    /// Appends one value after the last element.
    #[inline]
    pub fn push(&mut self, value: Value) {
        self.storage.push(value);
    }

    /// Removes and returns the last element, or `None` when empty.
    #[inline]
    pub fn pop(&mut self) -> Option<Value> {
        self.storage.pop()
    }

    /// Inserts one value before the first element.
    #[inline]
    pub fn unshift(&mut self, value: Value) {
        self.storage.unshift(value);
    }

    /// Removes and returns the first element, or `None` when empty.
    #[inline]
    pub fn shift(&mut self) -> Option<Value> {
        self.storage.shift()
    }

    /// Borrows an array payload after checking both semantic identity and payload type.
    pub fn from_value(value: &Value) -> Result<&Self, CoreError> {
        if value.array_type().is_none() {
            return Err(invalid_array_value());
        }
        value.downcast_ref::<Self>().ok_or_else(invalid_array_value)
    }

    /// Mutably borrows an exclusively owned array payload after validation.
    pub fn from_value_mut(value: &mut Value) -> Result<&mut Self, CoreError> {
        if value.array_type().is_none() {
            return Err(invalid_array_value());
        }
        value.downcast_mut::<Self>().ok_or_else(invalid_array_value)
    }
}

impl Clone for ArrayValue {
    /// Copies only the live window into a compact new allocation.
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
        }
    }
}

/// Returns the stable error for a scalar identity or malformed array payload.
pub(crate) fn invalid_array_value() -> CoreError {
    CoreError::InvalidValueRepresentation("array".into())
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
