use super::{Expression, FrameLiteralColumn};
use crate::syntax::{
    BinaryOperator, ComparisonOperator, LogicalOperator, SourceIdentifier, Span, UnaryOperator,
};

/// Creates a concise source span for expression fixtures.
fn span(start: usize, end: usize) -> Span {
    Span { start, end }
}

/// Creates a numeric expression with the requested span.
fn number_expression(span: Span) -> Expression {
    Expression::Number {
        raw_text: "42".into(),
        suffix: None,
        span,
    }
}

/// Verifies that every expression variant reports its stored source span.
#[test]
fn every_expression_variant_returns_its_span() {
    let expected_span = span(2, 11);
    let expressions = vec![
        number_expression(expected_span),
        Expression::String {
            value: "hello".into(),
            span: expected_span,
        },
        Expression::Boolean {
            raw_text: "true".into(),
            span: expected_span,
        },
        Expression::Variable {
            name: "distance".into(),
            span: expected_span,
        },
        Expression::FieldAccess {
            expression: Box::new(Expression::Variable {
                name: "employee".into(),
                span: expected_span,
            }),
            field: "name".into(),
            span: expected_span,
        },
        Expression::FrameLiteral {
            columns: vec![FrameLiteralColumn {
                name: "value".into(),
                values: vec![number_expression(expected_span)],
                span: expected_span,
            }],
            span: expected_span,
        },
        Expression::Unary {
            operator: UnaryOperator::Negation,
            operand: Box::new(number_expression(expected_span)),
            span: expected_span,
        },
        Expression::Binary {
            operator: BinaryOperator::Multiplication,
            left_operand: Box::new(number_expression(expected_span)),
            right_operand: Box::new(number_expression(expected_span)),
            span: expected_span,
        },
        Expression::Comparison {
            operator: ComparisonOperator::Less,
            left_operand: Box::new(number_expression(expected_span)),
            right_operand: Box::new(number_expression(expected_span)),
            span: expected_span,
        },
        Expression::Logical {
            operator: LogicalOperator::And,
            left_operand: Box::new(Expression::Boolean {
                raw_text: "true".into(),
                span: expected_span,
            }),
            right_operand: Box::new(Expression::Boolean {
                raw_text: "false".into(),
                span: expected_span,
            }),
            span: expected_span,
        },
        Expression::Convert {
            expression: Box::new(number_expression(expected_span)),
            target: "meters".into(),
            span: expected_span,
        },
        Expression::Call {
            namespace: None,
            function: SourceIdentifier {
                name: "print".into(),
                span: expected_span,
            },
            arguments: vec![number_expression(expected_span)],
            span: expected_span,
        },
    ];
    for expression in expressions {
        assert_eq!(expression.span(), expected_span);
    }
}
