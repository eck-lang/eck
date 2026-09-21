//! Array contracts and built-in operation vocabulary.

use std::sync::Arc;

use crate::semantic::{ScalarRepresentation, SemanticType};

/// One operation at either end of a mutable array.
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

/// The contract enforced when a value enters one array.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ArrayElementContract {
    /// Accept every concrete ECK value without conversion.
    Dynamic,
    /// Enforce one structural element type and scalar representation policy.
    Static(StaticArrayElementContract),
}

/// The type and representation promised by a statically constrained array.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StaticArrayElementContract {
    pub element: SemanticType,
    pub element_representation: ScalarRepresentation,
}

/// Describes the element contract owned by one array value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArrayType {
    pub element_contract: ArrayElementContract,
}

impl ArrayType {
    /// Creates an unconstrained heterogeneous array contract.
    pub fn dynamic() -> Self {
        Self {
            element_contract: ArrayElementContract::Dynamic,
        }
    }

    /// Creates a statically constrained array contract.
    pub fn static_element(
        semantic_type: SemanticType,
        representation: ScalarRepresentation,
    ) -> Self {
        Self {
            element_contract: ArrayElementContract::Static(StaticArrayElementContract {
                element: semantic_type,
                element_representation: representation,
            }),
        }
    }

    /// Returns the static element type, or `None` for a dynamic array.
    pub fn static_semantic_type(&self) -> Option<&SemanticType> {
        match &self.element_contract {
            ArrayElementContract::Dynamic => None,
            ArrayElementContract::Static(contract) => Some(&contract.element),
        }
    }

    /// Returns the scalar representation policy of a static array.
    pub fn static_representation(&self) -> Option<ScalarRepresentation> {
        match &self.element_contract {
            ArrayElementContract::Dynamic => None,
            ArrayElementContract::Static(contract) => Some(contract.element_representation),
        }
    }

    /// Returns a shared array contract without exposing storage details.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}
