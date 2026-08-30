use crate::{BinaryOperator, ComparisonOperator, Span, UnaryOperator};

#[derive(Clone, Debug)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug)]
pub enum Statement {
    VariableDecl {
        name: String,
        type_name: String,
        expression: Expression,
        span: Span,
    },
    Expression(Expression),
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
