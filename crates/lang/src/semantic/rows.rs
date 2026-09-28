//! Structural row and lazy iterable contracts.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::SemanticType;

/// One named field in a structural row contract.
#[derive(Clone, Debug)]
pub struct RowField {
    pub name: String,
    pub semantic_type: SemanticType,
    /// Source spelling retained for a failure diagnostic.
    pub source_type_name: String,
}

impl PartialEq for RowField {
    /// Compares structural field identity independently of diagnostic spelling.
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.semantic_type == other.semantic_type
    }
}

impl Eq for RowField {}

impl Hash for RowField {
    /// Hashes only the field name and resolved semantic type.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.semantic_type.hash(state);
    }
}

/// The ordered fields yielded by a typed source.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RowType {
    pub fields: Arc<[RowField]>,
}

impl RowType {
    /// Finds a field's stable slot before the row loop executes.
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|field| field.name == name)
    }
}

/// Describes the rows of a lazy source without implying materialization.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceType {
    pub row: Option<Arc<RowType>>,
}
