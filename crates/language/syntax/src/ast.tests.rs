use super::Expression;
use crate::{BinaryOperator, ComparisonOperator, Span, UnaryOperator};

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
        Expression::Convert {
            expression: Box::new(number_expression(expected_span)),
            target: "meters".into(),
            span: expected_span,
        },
        Expression::Call {
            name: "print".into(),
            arguments: vec![number_expression(expected_span)],
            span: expected_span,
        },
    ];
    for expression in expressions {
        assert_eq!(expression.span(), expected_span);
    }
}
