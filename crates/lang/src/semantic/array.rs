//! Array vocabulary shared by the compiler, IR, and runtime.
//!
//! These types describe an array's identity rather than its storage.

use crate::semantic::ValueType;

/// One operation at either end of a mutable array.
///
/// The compiler resolves a source spelling to one of these operations, and the
/// runtime applies it to the payload. `append` names [`ArrayEndOperation::Push`]
/// and `prepend` names [`ArrayEndOperation::Unshift`], so an alias never needs
/// a second implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayEndOperation {
    /// Adds one value after the last element (`push`, `append`).
    Push,
    /// Removes and returns the last element, or null when the array is empty.
    Pop,
    /// Adds one value before the first element (`unshift`, `prepend`).
    Unshift,
    /// Removes and returns the first element, or null when the array is empty.
    Shift,
}

impl ArrayEndOperation {
    /// Returns the canonical source name of this operation.
    pub fn name(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::Pop => "pop",
            Self::Unshift => "unshift",
            Self::Shift => "shift",
        }
    }

    /// Reports whether this operation removes one element.
    pub fn removes_element(self) -> bool {
        matches!(self, Self::Pop | Self::Shift)
    }
}

/// Selects the representation contract of an array's elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArrayElementMode {
    /// Keeps every element in the array's declared representation.
    Exact,
    /// Allows adaptive `int` elements to retain a wider integer representation.
    AdaptiveInt,
}

/// Describes the element contract of one array value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ArrayType {
    /// The declared or inferred common element type.
    pub element: ValueType,
    /// The representation contract applied when a value enters the array.
    pub element_mode: ArrayElementMode,
}
