use language_core::{BinaryOperator, CoreError, Registry, TypeDescriptor, Value};
use syntax::{Expression, Program, Span, Statement};

use crate::{TypedExpressionKind, TypedStatement};

use crate::compile;

fn parse_number(raw_text: &str, type_id: language_core::TypeId) -> Result<Value, CoreError> {
    Ok(Value::new(type_id, raw_text.to_owned()))
}

fn parse_boolean(raw_text: &str, type_id: language_core::TypeId) -> Result<Value, CoreError> {
    match raw_text {
        "true" => Ok(Value::new(type_id, true)),
        "false" => Ok(Value::new(type_id, false)),
        _ => Err(CoreError::InvalidLiteral {
            raw_text: raw_text.into(),
            type_name: "bool".into(),
            message: "expected `true` or `false`".into(),
        }),
    }
}

fn evaluate_boolean(value: &Value) -> Result<bool, CoreError> {
    value
        .downcast_ref::<bool>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("bool".into()))
}

fn format_value(value: &Value) -> Result<String, CoreError> {
    value
        .downcast_ref::<String>()
        .cloned()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("test number".into()))
}

fn execute_power(_: &Value, _: &Value) -> Result<Value, CoreError> {
    unreachable!("compiler tests do not execute registered operators")
}

fn span() -> Span {
    Span { start: 0, end: 1 }
}

#[test]
fn negative_integer_power_uses_the_default_fractional_type() {
    let mut registry = Registry::new();
    let integer = registry.allocate_type_id();
    let decimal = registry.allocate_type_id();
    for (id, name) in [(integer, "int"), (decimal, "decimal")] {
        registry
            .register_type(TypeDescriptor {
                id,
                name,
                is_integer: name == "int",
                parse_numeric_literal: Some(parse_number),
                parse_string_literal: None,
                parse_regex_literal: None,
                parse_boolean_literal: None,
                parse_null_literal: None,
                format: format_value,
            })
            .unwrap();
    }
    registry.set_default_integer(integer).unwrap();
    registry.set_default_fractional(decimal).unwrap();
    registry
        .register_binary_operator(
            BinaryOperator::Subtraction,
            decimal,
            decimal,
            decimal,
            execute_power,
        )
        .unwrap();
    registry
        .register_binary_operator(
            BinaryOperator::Power,
            integer,
            integer,
            integer,
            execute_power,
        )
        .unwrap();
    registry
        .register_binary_operator(
            BinaryOperator::Power,
            decimal,
            decimal,
            decimal,
            execute_power,
        )
        .unwrap();

    let program = Program {
        statements: vec![Statement::VariableDeclaration {
            name: "reciprocal".into(),
            type_name: "decimal".into(),
            expression: Expression::Binary {
                operator: syntax::BinaryOperator::Power,
                left_operand: Box::new(Expression::Number {
                    raw_text: "2".into(),
                    suffix: None,
                    span: span(),
                }),
                right_operand: Box::new(Expression::Unary {
                    operator: syntax::UnaryOperator::Negation,
                    operand: Box::new(Expression::Number {
                        raw_text: "1".into(),
                        suffix: None,
                        span: span(),
                    }),
                    span: span(),
                }),
                span: span(),
            },
            span: span(),
        }],
    };

    let typed = compile(&program, &registry).unwrap();
    let statement = &typed.statements[0];
    let TypedStatement::VariableDeclaration { value_type, .. } = statement else {
        panic!("expected a variable declaration");
    };

    assert_eq!(value_type.base, decimal);
}

#[test]
fn multiplication_by_literal_true_is_removed_from_the_typed_program() {
    let mut registry = Registry::new();
    let integer = registry.allocate_type_id();
    let boolean = registry.allocate_type_id();
    registry
        .register_type(TypeDescriptor {
            id: integer,
            name: "int",
            is_integer: true,
            parse_numeric_literal: Some(parse_number),
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_value,
        })
        .unwrap();
    registry
        .register_type(TypeDescriptor {
            id: boolean,
            name: "bool",
            is_integer: false,
            parse_numeric_literal: None,
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: Some(parse_boolean),
            parse_null_literal: None,
            format: format_value,
        })
        .unwrap();
    registry.set_default_integer(integer).unwrap();
    registry
        .set_default_boolean(boolean, evaluate_boolean)
        .unwrap();
    registry
        .register_binary_operator(
            BinaryOperator::Multiplication,
            integer,
            boolean,
            integer,
            execute_power,
        )
        .unwrap();

    let program = Program {
        statements: vec![Statement::VariableDeclaration {
            name: "result".into(),
            type_name: "int".into(),
            expression: Expression::Binary {
                operator: syntax::BinaryOperator::Multiplication,
                left_operand: Box::new(Expression::Number {
                    raw_text: "7".into(),
                    suffix: None,
                    span: span(),
                }),
                right_operand: Box::new(Expression::Boolean {
                    raw_text: "true".into(),
                    span: span(),
                }),
                span: span(),
            },
            span: span(),
        }],
    };

    let typed = compile(&program, &registry).unwrap();
    let TypedStatement::VariableDeclaration { expression, .. } = &typed.statements[0] else {
        panic!("expected a variable declaration");
    };

    assert!(matches!(expression.kind, TypedExpressionKind::Literal(_)));
}
