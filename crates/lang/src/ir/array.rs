//! Compiled array operation protocol.

/// One end operation selected by the compiler for runtime dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayMethod {
    /// Adds one value after the last element (`push`, `append`).
    Push,
    /// Removes and returns the last element, or null when the array is empty.
    Pop,
    /// Adds one value before the first element (`unshift`, `prepend`).
    Unshift,
    /// Removes and returns the first element, or null when the array is empty.
    Shift,
}

impl ArrayMethod {
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
