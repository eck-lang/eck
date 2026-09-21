//! Structural semantic types shared by the compiler, IR, and runtime.

use std::sync::Arc;

use crate::semantic::{ArrayType, ValueType};

/// Selects how a scalar value is represented when it crosses an array boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScalarRepresentation {
    /// Store and validate the exact scalar identity.
    #[default]
    Exact,
    /// Allow signed integer values to retain their concrete widening identity.
    AdaptiveSignedInteger,
}

impl ScalarRepresentation {
    /// Joins two representation promises for a value that may have either one.
    pub fn join(self, other: Self) -> Self {
        if matches!(
            (self, other),
            (Self::AdaptiveSignedInteger, _) | (_, Self::AdaptiveSignedInteger)
        ) {
            Self::AdaptiveSignedInteger
        } else {
            Self::Exact
        }
    }
}

/// Describes the complete semantic shape of an expression or binding.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SemanticType {
    /// One scalar value with a base type and optional subtype.
    Scalar(ValueType),
    /// One recursively described array value.
    Array(Arc<ArrayType>),
    /// One canonical finite union of structural types.
    Union(Arc<[SemanticType]>),
}

impl SemanticType {
    /// Builds an array semantic type from either an owned or shared contract.
    pub fn array(array_type: impl Into<Arc<ArrayType>>) -> Self {
        Self::Array(array_type.into())
    }

    /// Returns the concrete scalar identity when this type is scalar.
    pub fn as_scalar(&self) -> Option<ValueType> {
        match self {
            Self::Scalar(value_type) => Some(*value_type),
            _ => None,
        }
    }

    /// Builds a canonical union, flattening nested unions and removing duplicates.
    pub fn union(members: impl IntoIterator<Item = SemanticType>) -> Self {
        let mut flattened = Vec::new();
        for member in members {
            match member {
                Self::Union(nested) => flattened.extend(nested.iter().cloned()),
                member => flattened.push(member),
            }
        }
        flattened.sort_by_key(Self::canonical_key);
        flattened.dedup();
        match flattened.len() {
            0 => panic!("a semantic union must contain at least one member"),
            1 => flattened.pop().expect("one union member was retained"),
            _ => Self::Union(flattened.into_boxed_slice().into()),
        }
    }

    /// Returns whether this type contains the given scalar identity.
    pub fn contains_scalar(&self, value_type: ValueType) -> bool {
        match self {
            Self::Scalar(actual) => *actual == value_type,
            Self::Array(_) => false,
            Self::Union(members) => members
                .iter()
                .any(|member| member.contains_scalar(value_type)),
        }
    }

    /// Removes one scalar member from a union and canonicalizes the result.
    pub fn without_scalar(&self, value_type: ValueType) -> Option<Self> {
        match self {
            Self::Scalar(actual) if *actual == value_type => None,
            Self::Scalar(_) | Self::Array(_) => Some(self.clone()),
            Self::Union(members) => {
                let remaining = members
                    .iter()
                    .filter_map(|member| member.without_scalar(value_type))
                    .collect::<Vec<_>>();
                (!remaining.is_empty()).then(|| Self::union(remaining))
            }
        }
    }

    /// Returns the deterministic key used for canonical union ordering.
    fn canonical_key(&self) -> String {
        match self {
            Self::Scalar(value_type) => format!("scalar:{value_type:?}"),
            Self::Array(array_type) => format!("array:{array_type:?}"),
            Self::Union(members) => format!("union:{members:?}"),
        }
    }
}

/// Reports whether a source semantic type can be stored in a destination type.
///
/// Mutable arrays are invariant: their complete recursive contracts must match.
/// A source union must be valid for every alternative it may carry, while a
/// destination union accepts a source that fits at least one member.
pub fn is_assignable(source: &SemanticType, destination: &SemanticType) -> bool {
    match (source, destination) {
        (SemanticType::Union(source_members), SemanticType::Union(destination_members)) => {
            source_members.iter().all(|source_member| {
                destination_members
                    .iter()
                    .any(|destination_member| is_assignable(source_member, destination_member))
            })
        }
        (_, SemanticType::Union(destination_members)) => destination_members
            .iter()
            .any(|member| is_assignable(source, member)),
        (SemanticType::Union(source_members), _) => source_members
            .iter()
            .all(|member| is_assignable(member, destination)),
        (SemanticType::Scalar(source), SemanticType::Scalar(destination)) => {
            source.base == destination.base
                && (destination.subtype.is_none() || source.subtype == destination.subtype)
        }
        (SemanticType::Array(source), SemanticType::Array(destination)) => source == destination,
        _ => false,
    }
}

/// Describes a resolved source type together with its scalar storage policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredType {
    /// The structural type after aliases, nullable lowering, and unions resolve.
    pub semantic_type: SemanticType,
    /// The representation policy carried by scalar leaves of the declaration.
    pub representation: ScalarRepresentation,
}

#[cfg(test)]
#[path = "types.tests.rs"]
mod tests;
