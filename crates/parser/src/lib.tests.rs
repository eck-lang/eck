use super::*;
use syntax::{BinaryOperator, Expression, Statement, UnaryOperator};

#[test]
fn parses_remainder_and_power() {
    let program = parse("print(10 % 3 + 2 ** 3)\n").unwrap();
    let Statement::Expression(Expression::Call { arguments, .. }) = &program.statements[0] else {
        panic!("expected a call expression");
    };
    let Expression::Binary {
        operator,
        left_operand,
        right_operand,
        ..
    } = &arguments[0]
    else {
        panic!("expected an addition expression");
    };
    assert_eq!(*operator, BinaryOperator::Addition);

    let Expression::Binary { operator, .. } = left_operand.as_ref() else {
        panic!("expected a remainder expression");
    };
    assert_eq!(*operator, BinaryOperator::Remainder);

    let Expression::Binary { operator, .. } = right_operand.as_ref() else {
        panic!("expected a power expression");
    };
    assert_eq!(*operator, BinaryOperator::Power);
}

#[test]
fn power_is_right_associative() {
    let program = parse("print(2 ** 3 ** 2)\n").unwrap();
    let Statement::Expression(Expression::Call { arguments, .. }) = &program.statements[0] else {
        panic!("expected a call expression");
    };
    let Expression::Binary {
        left_operand,
        right_operand,
        ..
    } = &arguments[0]
    else {
        panic!("expected a power expression");
    };
    assert!(
        matches!(left_operand.as_ref(), Expression::Number { raw_text, .. } if raw_text == "2")
    );
    assert!(matches!(
        right_operand.as_ref(),
        Expression::Binary {
            operator: BinaryOperator::Power,
            ..
        }
    ));
}

#[test]
fn parses_a_negative_power_exponent_as_negation() {
    let program = parse("print(2 ** -1)\n").unwrap();
    let Statement::Expression(Expression::Call { arguments, .. }) = &program.statements[0] else {
        panic!("expected a call expression");
    };
    let Expression::Binary { right_operand, .. } = &arguments[0] else {
        panic!("expected a power expression");
    };
    assert!(matches!(
        right_operand.as_ref(),
        Expression::Unary {
            operator: UnaryOperator::Negation,
            operand,
            ..
        } if matches!(operand.as_ref(), Expression::Number { raw_text, .. } if raw_text == "1")
    ));
}

#[test]
fn parses_negation_of_variables_and_parenthesized_expressions() {
    let program = parse("print(-value, -(value + 1), -(-value))\n").unwrap();
    let Statement::Expression(Expression::Call { arguments, .. }) = &program.statements[0] else {
        panic!("expected a call expression");
    };
    assert!(matches!(arguments[0], Expression::Unary { .. }));
    assert!(matches!(arguments[1], Expression::Unary { .. }));
    assert!(matches!(arguments[2], Expression::Unary { .. }));
}

#[test]
fn rejects_double_minus_to_reserve_pre_decrement_syntax() {
    let error = parse("print(--value)\n").unwrap_err();

    assert!(error.message.contains("`--` is not supported"));
}

#[test]
fn parses_adjacent_numeric_suffix() {
    let program = parse("distance: decimal = 10m\n").unwrap();
    let Statement::VariableDecl { expression, .. } = &program.statements[0] else {
        panic!("expected a variable declaration");
    };
    assert!(matches!(
        expression,
        Expression::Number {
            raw_text,
            suffix: Some(suffix),
            ..
        } if raw_text == "10" && suffix == "m"
    ));
}

#[test]
fn parses_boolean_literals_without_claiming_identifiers_with_the_same_prefix() {
    let program = parse("enabled: bool = true\nprint(false)\ntruthy: int = 1\n").unwrap();

    let Statement::VariableDecl { expression, .. } = &program.statements[0] else {
        panic!("expected a variable declaration");
    };
    assert!(matches!(
        expression,
        Expression::Boolean { raw_text, .. } if raw_text == "true"
    ));

    let Statement::Expression(Expression::Call { arguments, .. }) = &program.statements[1] else {
        panic!("expected a call expression");
    };
    assert!(matches!(
        &arguments[0],
        Expression::Boolean { raw_text, .. } if raw_text == "false"
    ));

    let Statement::VariableDecl { name, .. } = &program.statements[2] else {
        panic!("expected a variable declaration");
    };
    assert_eq!(name, "truthy");
}

#[test]
fn parses_space_separated_numeric_suffix() {
    let program = parse("distance: decimal = 10 meters\n").unwrap();
    let Statement::VariableDecl { expression, .. } = &program.statements[0] else {
        panic!("expected a variable declaration");
    };
    assert!(matches!(
        expression,
        Expression::Number {
            raw_text,
            suffix: Some(suffix),
            ..
        } if raw_text == "10" && suffix == "meters"
    ));
}

#[test]
fn parses_postfix_measure_conversion() {
    let program = parse("print(distance->km)\n").unwrap();
    let Statement::Expression(Expression::Call { arguments, .. }) = &program.statements[0] else {
        panic!("expected a call expression");
    };
    assert!(matches!(
        &arguments[0],
        Expression::Convert { expression, target, .. }
            if target == "km" && matches!(expression.as_ref(), Expression::Variable { name, .. } if name == "distance")
    ));
}
