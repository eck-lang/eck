use super::Expression;
use crate::Span;

/// Represents one compile-time namespace import declaration.
#[derive(Clone, Debug)]
pub struct UseDeclaration {
    pub clause: UseClause,
    pub use_span: Span,
    pub span: Span,
}

/// Captures the three semantically distinct namespace import forms.
#[derive(Clone, Debug)]
pub enum UseClause {
    Namespace {
        namespace: SourceIdentifier,
        alias: Option<SourceIdentifier>,
    },
    Members {
        namespace: SourceIdentifier,
        members: Vec<UseMember>,
    },
    Wildcard {
        namespace: SourceIdentifier,
        alias: Option<SourceIdentifier>,
    },
}

/// Stores one selectively imported member and its optional local alias.
#[derive(Clone, Debug)]
pub struct UseMember {
    pub name: SourceIdentifier,
    pub alias: Option<SourceIdentifier>,
    pub span: Span,
}

/// Stores an identifier together with its exact source span.
#[derive(Clone, Debug)]
pub struct SourceIdentifier {
    pub name: String,
    pub span: Span,
}

/// Describes the logical fields that make up one user-defined row type.
#[derive(Clone, Debug)]
pub struct TypeDefinition {
    pub name: String,
    pub fields: Vec<TypeField>,
    pub span: Span,
}

/// Describes one named field in a user-defined row type.
#[derive(Clone, Debug)]
pub struct TypeField {
    pub name: String,
    pub type_name: String,
    pub span: Span,
}

/// Describes a reusable relation independently of concrete frame instances.
#[derive(Clone, Debug)]
pub struct RelationDefinition {
    pub name: String,
    pub roles: Vec<RelationRole>,
    pub predicates: Vec<Expression>,
    pub span: Span,
}

/// Describes one named participant in a relation definition.
#[derive(Clone, Debug)]
pub struct RelationRole {
    pub name: String,
    pub row_type_name: String,
    pub cardinality: RelationCardinality,
    pub span: Span,
}

/// Describes the expected number of matching rows for a relation role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationCardinality {
    One,
    Many,
}

/// Connects one relation definition to explicitly named frame instances.
#[derive(Clone, Debug)]
pub struct RelationBinding {
    pub name: String,
    pub definition_name: String,
    pub roles: Vec<RelationRoleBinding>,
    pub span: Span,
}

/// Connects one relation role to one concrete frame variable.
#[derive(Clone, Debug)]
pub struct RelationRoleBinding {
    pub role_name: String,
    pub frame_name: String,
    pub span: Span,
}

/// Stores one named entry inside a source configuration object.
#[derive(Clone, Debug)]
pub struct ConfigurationEntry {
    pub name: String,
    pub value: ConfigurationValue,
    pub span: Span,
}

/// Represents the source forms accepted inside `@config` objects.
#[derive(Clone, Debug)]
pub enum ConfigurationValue {
    Number {
        raw_text: String,
        span: Span,
    },
    Symbol {
        name: String,
        span: Span,
    },
    Object {
        entries: Vec<ConfigurationEntry>,
        span: Span,
    },
}

impl ConfigurationValue {
    /// Returns the byte range occupied by this configuration value.
    pub fn span(&self) -> Span {
        match self {
            Self::Number { span, .. } | Self::Symbol { span, .. } | Self::Object { span, .. } => {
                *span
            }
        }
    }
}
