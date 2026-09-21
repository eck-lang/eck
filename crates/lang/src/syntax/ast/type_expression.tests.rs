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

/// Verifies display adds parentheses only where postfix precedence requires them.
#[test]
fn renders_union_and_recursive_array_precedence() {
    let named = |name: &str| TypeExpression::Named {
        name: name.into(),
        span: Span { start: 0, end: 0 },
    };
    let union = TypeExpression::Union {
        members: vec![named("int"), named("string")],
        span: Span { start: 0, end: 0 },
    };
    let array_of_union = TypeExpression::Array {
        element: Box::new(union.clone()),
        span: Span { start: 0, end: 0 },
    };
    let union_of_arrays = TypeExpression::Union {
        members: vec![
            TypeExpression::Array {
                element: Box::new(named("int")),
                span: Span { start: 0, end: 0 },
            },
            TypeExpression::Array {
                element: Box::new(named("string")),
                span: Span { start: 0, end: 0 },
            },
        ],
        span: Span { start: 0, end: 0 },
    };

    assert_eq!(array_of_union.to_string(), "(int | string)[]");
    assert_eq!(union_of_arrays.to_string(), "int[] | string[]");
    assert_eq!(
        TypeExpression::Nullable {
            inner: Box::new(array_of_union),
            span: Span { start: 0, end: 0 },
        }
        .to_string(),
        "(int | string)[]?"
    );
}

/// Verifies repeated array and nullable postfix operators preserve source order.
#[test]
fn renders_all_postfix_combinations() {
    let named = || TypeExpression::Named {
        name: "int".into(),
        span: Span { start: 0, end: 0 },
    };
    let array = |element| TypeExpression::Array {
        element: Box::new(element),
        span: Span { start: 0, end: 0 },
    };
    let nullable = |inner| TypeExpression::Nullable {
        inner: Box::new(inner),
        span: Span { start: 0, end: 0 },
    };

    assert_eq!(array(nullable(named())).to_string(), "int?[]");
    assert_eq!(nullable(array(named())).to_string(), "int[]?");
    assert_eq!(nullable(array(nullable(named()))).to_string(), "int?[]?");
    assert_eq!(nullable(array(array(named()))).to_string(), "int[][]?");
}
