use super::*;

/// Verifies postfix type nodes retain their complete source span and spelling.
#[test]
fn reports_nested_type_expression_span_and_source() {
    let expression = TypeExpression::Nullable {
        inner: Box::new(TypeExpression::Array {
            element: Box::new(TypeExpression::Qualified {
                base: Box::new(TypeExpression::Named {
                    name: "int".into(),
                    span: Span { start: 10, end: 13 },
                }),
                subtype: "mm".into(),
                span: Span { start: 10, end: 18 },
            }),
            span: Span { start: 10, end: 20 },
        }),
        span: Span { start: 10, end: 21 },
    };

    assert_eq!(expression.span(), Span { start: 10, end: 21 });
    assert_eq!(expression.to_string(), "int<mm>[]?");
}
