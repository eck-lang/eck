use super::*;
use crate::parser::lexer::lex;
use crate::syntax::{BinaryOperator, ComparisonOperator, Expression, LogicalOperator, Statement};

fn parse_expression(source: &str) -> Expression {
    let mut parser = Parser::new(lex(source).unwrap());
    parser.parse_expression(0).unwrap()
}

/// Verifies expression keys, multiline entries, and a trailing comma.
#[test]
fn parses_map_literals_with_expression_keys_and_trailing_commas() {
    let expression = parse_expression("{\n  \"name\": 1 + offset,\n  index + 1: values[0],\n}");

    let Expression::MapLiteral { entries, span } = expression else {
        panic!("expected a map literal");
    };
    assert_eq!(entries.len(), 2);
    assert!(matches!(
        &entries[0].0,
        Expression::String { value, .. } if value == "name"
    ));
    assert!(matches!(
        &entries[0].1,
        Expression::Binary {
            operator: BinaryOperator::Addition,
            ..
        }
    ));
    assert!(matches!(
        &entries[1].0,
        Expression::Binary {
            operator: BinaryOperator::Addition,
            ..
        }
    ));
    assert!(matches!(&entries[1].1, Expression::ElementAccess { .. }));
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 49);
}

/// Verifies that a map literal may contain no entries.
#[test]
fn parses_empty_map_literals() {
    assert!(matches!(
        parse_expression("{}"),
        Expression::MapLiteral { entries, .. } if entries.is_empty()
    ));
}

/// Verifies that map literals require an explicit colon after each key.
#[test]
fn rejects_identifier_shorthand_in_map_literals() {
    let mut parser = Parser::new(lex("{name}").unwrap());
    let error = parser.parse_expression(0).unwrap_err();

    assert!(error.message.contains("expected Colon"));
}

/// Verifies that a statement beginning with a brace remains a block.
#[test]
fn keeps_a_leading_brace_statement_as_a_block() {
    let mut parser = Parser::new(lex("{ print(1) }").unwrap());
    let program = parser.parse_program().unwrap();

    assert!(matches!(
        program.statements.as_slice(),
        [Statement::Block(_)]
    ));
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
    let expression = parse_expression("distance->to(cm) + 1cm >= 101cm");
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
        matches!(left_operand.as_ref(), Expression::Binary { operator: BinaryOperator::Addition, left_operand, .. } if matches!(left_operand.as_ref(), Expression::Pipe { function, .. } if function == "to"))
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

/// Verifies prefix `!` produces a logical-not unary expression.
#[test]
fn parses_logical_not_expressions() {
    let expression = parse_expression("!enabled");

    assert!(matches!(
        expression,
        Expression::Unary {
            operator: UnaryOperator::LogicalNot,
            operand,
            ..
        } if matches!(operand.as_ref(), Expression::Variable { name, .. } if name == "enabled")
    ));
}

/// Verifies prefix `!` binds its operand tighter than a postfix element access.
#[test]
fn parses_logical_not_of_element_access() {
    let expression = parse_expression("!values[0]");

    assert!(matches!(
        expression,
        Expression::Unary {
            operator: UnaryOperator::LogicalNot,
            operand,
            ..
        } if matches!(operand.as_ref(), Expression::ElementAccess { .. })
    ));
}
