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
    /// Describes a value that may have one of several structural types.
    Union {
        members: Vec<TypeExpression>,
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
            | Self::Nullable { span, .. }
            | Self::Union { span, .. } => *span,
        }
    }
}

impl fmt::Display for TypeExpression {
    /// Renders this syntax tree in the source spelling used by diagnostics and tooling.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format_with_precedence(formatter, 0)
    }
}

impl TypeExpression {
    /// Renders one type expression while adding parentheses required by postfix precedence.
    fn format_with_precedence(
        &self,
        formatter: &mut fmt::Formatter<'_>,
        parent_precedence: u8,
    ) -> fmt::Result {
        let precedence = match self {
            Self::Union { .. } => 1,
            Self::Array { .. } | Self::Nullable { .. } => 2,
            Self::Named { .. } | Self::Qualified { .. } => 3,
        };
        let parenthesized = precedence < parent_precedence;
        if parenthesized {
            formatter.write_str("(")?;
        }
        match self {
            Self::Named { name, .. } => formatter.write_str(name)?,
            Self::Qualified { base, subtype, .. } => {
                base.format_with_precedence(formatter, 3)?;
                write!(formatter, "<{subtype}>")?;
            }
            Self::Array { element, .. } => {
                element.format_with_precedence(formatter, 2)?;
                formatter.write_str("[]")?;
            }
            Self::Nullable { inner, .. } => {
                inner.format_with_precedence(formatter, 2)?;
                formatter.write_str("?")?;
            }
            Self::Union { members, .. } => {
                for (index, member) in members.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(" | ")?;
                    }
                    member.format_with_precedence(formatter, 1)?;
                }
            }
        }
        if parenthesized {
            formatter.write_str(")")?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "type_expression.tests.rs"]
mod tests;
