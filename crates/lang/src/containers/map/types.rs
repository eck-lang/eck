//! Map container type vocabulary.

use crate::semantic::SemanticType;

/// The key and value contract enforced by one map.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MapEntryContract {
    /// Accepts every supported scalar key and every concrete ECK value.
    Dynamic,
    /// Retains recursive structural contracts for both keys and values.
    Static {
        key: Box<SemanticType>,
        value: Box<SemanticType>,
    },
}

/// Describes the key and value contract owned by one map value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MapType {
    pub entry_contract: MapEntryContract,
}

impl MapType {
    /// Creates an unconstrained associative map contract.
    pub const fn dynamic() -> Self {
        Self {
            entry_contract: MapEntryContract::Dynamic,
        }
    }

    /// Creates a recursively typed key/value map contract.
    pub fn static_entries(key: SemanticType, value: SemanticType) -> Self {
        Self {
            entry_contract: MapEntryContract::Static {
                key: Box::new(key),
                value: Box::new(value),
            },
        }
    }

    /// Returns the static key and value types, or `None` for a dynamic map.
    pub fn static_entry_types(&self) -> Option<(&SemanticType, &SemanticType)> {
        match &self.entry_contract {
            MapEntryContract::Dynamic => None,
            MapEntryContract::Static { key, value } => Some((key, value)),
        }
    }
}

impl Default for MapType {
    /// Returns the dynamic map contract.
    fn default() -> Self {
        Self::dynamic()
    }
}

#[cfg(test)]
#[path = "types.tests.rs"]
mod tests;
