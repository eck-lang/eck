//! Structural semantic types shared by the compiler, IR, and runtime.

use crate::semantic::{ArrayType, ValueType};

/// Describes the complete semantic shape of an expression or binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticType {
    /// One scalar value with a base type and optional subtype.
    Scalar(ValueType),
    /// One array value with a non-nested element contract.
    Array(ArrayType),
}

#[cfg(test)]
#[path = "types.tests.rs"]
mod tests;
