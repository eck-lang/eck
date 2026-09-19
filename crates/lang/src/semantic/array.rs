//! Array vocabulary shared by the compiler, IR, and runtime.
//!
//! These types describe an array's identity rather than its storage.

use crate::semantic::ValueType;

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
