use super::SourceIdentifier;
use crate::{BinaryOperator, ComparisonOperator, LogicalOperator, Span, UnaryOperator};

/// Stores one named typed column inside a hand-written frame literal.
#[derive(Clone, Debug)]
pub struct FrameLiteralColumn {
    pub name: String,
    pub values: Vec<Expression>,
    pub span: Span,
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
    Regex {
        raw_text: String,
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
    /// A source array literal whose element contract is inferred or supplied by a binding.
    ArrayLiteral {
        elements: Vec<Expression>,
        span: Span,
    },
    /// Zero-based postfix access to one array element.
    ElementAccess {
        expression: Box<Expression>,
        index: Box<Expression>,
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
    Pipe {
        expression: Box<Expression>,
        function: String,
        arguments: Vec<Expression>,
        span: Span,
    },
    Call {
        namespace: Option<SourceIdentifier>,
        function: SourceIdentifier,
        arguments: Vec<Expression>,
        span: Span,
    },
}

impl Expression {
    /// Returns the byte range occupied by this expression.
    pub fn span(&self) -> Span {
        match self {
            Expression::Number { span, .. }
            | Expression::String { span, .. }
            | Expression::Regex { span, .. }
            | Expression::Boolean { span, .. }
            | Expression::Null { span, .. }
            | Expression::Variable { span, .. }
            | Expression::FieldAccess { span, .. }
            | Expression::FrameLiteral { span, .. }
            | Expression::ArrayLiteral { span, .. }
            | Expression::ElementAccess { span, .. }
            | Expression::Unary { span, .. }
            | Expression::Binary { span, .. }
            | Expression::Comparison { span, .. }
            | Expression::Logical { span, .. }
            | Expression::Convert { span, .. }
            | Expression::Pipe { span, .. }
            | Expression::Call { span, .. } => *span,
        }
    }
}

#[cfg(test)]
#[path = "expression.tests.rs"]
mod tests;
