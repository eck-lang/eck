use crate::{BinaryOperator, Span, UnaryOperator};

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
            | Expression::Convert { span, .. }
            | Expression::Call { span, .. } => *span,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::Expression;
    use crate::{BinaryOperator, Span, UnaryOperator};

    fn span(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    fn number_expression(span: Span) -> Expression {
        Expression::Number {
            raw_text: "42".into(),
            suffix: None,
            span,
        }
    }

    #[test]
    fn number_expression_returns_its_span() {
        let expected_span = span(3, 5);
        let expression = number_expression(expected_span);

        assert_eq!(expression.span(), expected_span);
    }

    #[test]
    fn string_expression_returns_its_span() {
        let expected_span = span(8, 15);
        let expression = Expression::String {
            value: "hello".into(),
            span: expected_span,
        };

        assert_eq!(expression.span(), expected_span);
    }

    #[test]
    fn variable_expression_returns_its_span() {
        let expected_span = span(1, 9);
        let expression = Expression::Variable {
            name: "distance".into(),
            span: expected_span,
        };

        assert_eq!(expression.span(), expected_span);
    }

    #[test]
    fn unary_expression_returns_its_span() {
        let expected_span = span(2, 4);
        let expression = Expression::Unary {
            operator: UnaryOperator::Negation,
            operand: Box::new(number_expression(span(3, 4))),
            span: expected_span,
        };

        assert_eq!(expression.span(), expected_span);
    }

    #[test]
    fn binary_expression_returns_its_span() {
        let expected_span = span(2, 11);
        let expression = Expression::Binary {
            operator: BinaryOperator::Multiplication,
            left_operand: Box::new(number_expression(span(2, 3))),
            right_operand: Box::new(number_expression(span(10, 11))),
            span: expected_span,
        };

        assert_eq!(expression.span(), expected_span);
    }

    #[test]
    fn conversion_expression_returns_its_span() {
        let expected_span = span(4, 12);
        let expression = Expression::Convert {
            expression: Box::new(number_expression(span(4, 6))),
            target: "meters".into(),
            span: expected_span,
        };

        assert_eq!(expression.span(), expected_span);
    }

    #[test]
    fn call_expression_returns_its_span() {
        let expected_span = span(0, 9);
        let expression = Expression::Call {
            name: "print".into(),
            arguments: vec![number_expression(span(6, 8))],
            span: expected_span,
        };

        assert_eq!(expression.span(), expected_span);
    }
}
