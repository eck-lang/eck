use super::*;
use crate::lexer::lex;
use syntax::{BinaryOperator, ComparisonOperator, Expression, LogicalOperator};

fn parse_expression(source: &str) -> Expression {
    let mut parser = Parser::new(lex(source).unwrap());
    parser.parse_expression(0).unwrap()
}

#[test]
fn parses_an_adjacent_percent_as_a_numeric_suffix() {
    let expression = parse_expression("10%");

    assert!(matches!(
        expression,
        Expression::Number {
            raw_text,
            suffix: Some(suffix),
            ..
        } if raw_text == "10" && suffix == "%"
    ));
}

#[test]
fn keeps_spaced_percent_as_the_remainder_operator() {
    let expression = parse_expression("10 % 3");

    let Expression::Binary {
        operator,
        left_operand,
        right_operand,
        ..
    } = &expression
    else {
        panic!("expected a remainder expression");
    };
    assert_eq!(*operator, BinaryOperator::Remainder);
    assert!(matches!(
        left_operand.as_ref(),
        Expression::Number {
            raw_text,
            suffix: None,
            ..
        } if raw_text == "10"
    ));
    assert!(matches!(
        right_operand.as_ref(),
        Expression::Number {
            raw_text,
            suffix: None,
            ..
        } if raw_text == "3"
    ));
}

#[test]
fn comparisons_have_lower_precedence_than_arithmetic_and_conversions() {
    let expression = parse_expression("distance->cm + 1cm >= 101cm");
    let Expression::Comparison {
        operator,
        left_operand,
        ..
    } = expression
    else {
        panic!("expected a comparison expression");
    };
    assert_eq!(operator, ComparisonOperator::GreaterOrEqual);
    assert!(
        matches!(left_operand.as_ref(), Expression::Binary { operator: BinaryOperator::Addition, left_operand, .. } if matches!(left_operand.as_ref(), Expression::Convert { .. }))
    );
}

#[test]
fn accepts_parenthesized_comparisons_but_rejects_chained_ones() {
    let expression = parse_expression("(a < b) == (c < d)");
    assert!(matches!(
        expression,
        Expression::Comparison {
            operator: ComparisonOperator::Equal,
            ..
        }
    ));

    let mut parser = Parser::new(lex("a < b < c").unwrap());
    let error = parser.parse_expression(0).unwrap_err();
    assert!(error.message.contains("chained comparisons"));
}

/// Verifies role-qualified fields and boolean precedence used by relation predicates.
#[test]
fn parses_field_access_and_logical_predicates() {
    let expression = parse_expression(
        "orders.customer_id == customer.id || orders.company_id == customer.company_id && orders.id != customer.id",
    );
    let Expression::Logical {
        operator,
        right_operand,
        ..
    } = expression
    else {
        panic!("expected a logical expression");
    };
    assert_eq!(operator, LogicalOperator::Or);
    assert!(matches!(
        right_operand.as_ref(),
        Expression::Logical {
            operator: LogicalOperator::And,
            ..
        }
    ));
}
