use crate::{BinaryOperator, ComparisonOperator, Span, UnaryOperator};

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
    Variable {
        name: String,
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
            | Expression::Variable { span, .. }
            | Expression::Unary { span, .. }
            | Expression::Binary { span, .. }
            | Expression::Comparison { span, .. }
            | Expression::Convert { span, .. }
            | Expression::Call { span, .. } => *span,
        }
    }
}

#[cfg(test)]
#[path = "ast.tests.rs"]
mod tests;
