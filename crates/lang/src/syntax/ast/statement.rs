use super::{
    ConfigurationEntry, Expression, RelationBinding, RelationDefinition, TypeDefinition,
    TypeExpression, UseDeclaration,
};
use crate::syntax::Span;

#[derive(Clone, Debug)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

/// Declares whether a source binding may be reassigned after initialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKind {
    /// Introduces a mutable binding through the `let` keyword.
    Let,
    /// Introduces an immutable binding through the `const` keyword.
    Const,
}

#[derive(Clone, Debug)]
pub enum Statement {
    Use(UseDeclaration),
    TypeDeclaration {
        definition: TypeDefinition,
        span: Span,
    },
    FrameDeclaration {
        name: String,
        row_type_name: String,
        expression: Option<Expression>,
        span: Span,
    },
    RelationDefinition {
        definition: RelationDefinition,
        span: Span,
    },
    RelationBinding {
        binding: RelationBinding,
        span: Span,
    },
    Configuration {
        entries: Vec<ConfigurationEntry>,
        span: Span,
    },
    VariableDeclaration {
        name: String,
        type_name: String,
        expression: Expression,
        span: Span,
    },
    BindingDeclaration {
        kind: BindingKind,
        name: String,
        type_expression: Option<TypeExpression>,
        expression: Expression,
        span: Span,
    },
    Assignment {
        name: String,
        expression: Expression,
        span: Span,
    },
    /// Replaces one zero-based element of an existing mutable array binding.
    IndexedAssignment {
        name: String,
        index: Expression,
        expression: Expression,
        span: Span,
    },
    Block(Block),
    If {
        condition: Expression,
        body: Block,
        else_body: Option<Block>,
        span: Span,
    },
    While {
        condition: Expression,
        body: Block,
        span: Span,
    },
    For {
        variable: String,
        start: Expression,
        end: Expression,
        body: Block,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    Expression(Expression),
}
