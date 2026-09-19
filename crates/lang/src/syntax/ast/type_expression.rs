//! Structured syntax nodes for explicit binding type annotations.

use std::fmt;

use crate::syntax::Span;

/// Represents one parsed type annotation without resolving it to a registry type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeExpression {
    /// Names one base type such as `int` or `string`.
    Named { name: String, span: Span },
    /// Adds an element subtype constraint such as `<mm>` to a base type.
    Qualified {
        base: Box<TypeExpression>,
        subtype: String,
        span: Span,
    },
    /// Declares an array whose element type is represented by `element`.
    Array {
        element: Box<TypeExpression>,
        span: Span,
    },
    /// Marks the inner type as nullable.
    Nullable {
        inner: Box<TypeExpression>,
        span: Span,
    },
}

impl TypeExpression {
    /// Returns the complete source span occupied by this type expression.
    pub fn span(&self) -> Span {
        match self {
            Self::Named { span, .. }
            | Self::Qualified { span, .. }
            | Self::Array { span, .. }
            | Self::Nullable { span, .. } => *span,
        }
    }
}

impl fmt::Display for TypeExpression {
    /// Renders this syntax tree in the source spelling used by diagnostics and tooling.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named { name, .. } => formatter.write_str(name),
            Self::Qualified { base, subtype, .. } => write!(formatter, "{base}<{subtype}>",),
            Self::Array { element, .. } => write!(formatter, "{element}[]",),
            Self::Nullable { inner, .. } => write!(formatter, "{inner}?",),
        }
    }
}

#[cfg(test)]
#[path = "type_expression.tests.rs"]
mod tests;
