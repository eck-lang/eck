use crate::{BinaryOperator, ComparisonOperator, LogicalOperator, Span, UnaryOperator};

#[derive(Clone, Debug)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Statement {
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
    VariableDecl {
        name: String,
        type_name: String,
        expression: Expression,
        span: Span,
    },
    If {
        condition: Expression,
        body: Block,
        span: Span,
    },
    Expression(Expression),
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

/// Stores one named typed column inside a hand-written frame literal.
#[derive(Clone, Debug)]
pub struct FrameLiteralColumn {
    pub name: String,
    pub values: Vec<Expression>,
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

#[derive(Clone, Debug)]
pub enum Expression {
    Number {
        raw_text: String,
        suffix: Option<String>,
        span: Span,
    },
    String {
        value: String,
        span: Span,
    },
    Boolean {
        raw_text: String,
        span: Span,
    },
    Null {
        span: Span,
    },
    Variable {
        name: String,
        span: Span,
    },
    FieldAccess {
        expression: Box<Expression>,
        field: String,
        span: Span,
    },
    FrameLiteral {
        columns: Vec<FrameLiteralColumn>,
        span: Span,
    },
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
        span: Span,
    },
    Binary {
        operator: BinaryOperator,
        left_operand: Box<Expression>,
        right_operand: Box<Expression>,
        span: Span,
    },
    Comparison {
        operator: ComparisonOperator,
        left_operand: Box<Expression>,
        right_operand: Box<Expression>,
        span: Span,
    },
    Logical {
        operator: LogicalOperator,
        left_operand: Box<Expression>,
        right_operand: Box<Expression>,
        span: Span,
    },
    Convert {
        expression: Box<Expression>,
        target: String,
        span: Span,
    },
    Call {
        name: String,
        arguments: Vec<Expression>,
        span: Span,
    },
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::Number { span, .. }
            | Expression::String { span, .. }
            | Expression::Boolean { span, .. }
            | Expression::Null { span, .. }
            | Expression::Variable { span, .. }
            | Expression::FieldAccess { span, .. }
            | Expression::FrameLiteral { span, .. }
            | Expression::Unary { span, .. }
            | Expression::Binary { span, .. }
            | Expression::Comparison { span, .. }
            | Expression::Logical { span, .. }
            | Expression::Convert { span, .. }
            | Expression::Call { span, .. } => *span,
        }
    }
}

#[cfg(test)]
#[path = "ast.tests.rs"]
mod tests;
