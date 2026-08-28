mod error;
mod lexer;
mod parser;

pub use error::ParseError;

use syntax::Program;

/// Lexes and parses one ECK source file into its syntax tree.
///
/// The function performs no semantic validation: type, operator, and name
/// resolution remain responsibilities of the compiler layer.
pub fn parse(source: &str) -> Result<Program, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(tokens).parse_program()
}

#[cfg(test)]
mod tests {
    use super::*;
    use syntax::{BinaryOperator, Expression, Statement};

    #[test]
    fn parses_remainder_and_power() {
        let program = parse("print(10 % 3 + 2 ** 3)\n").unwrap();
        let Statement::Expression(Expression::Call { arguments, .. }) = &program.statements[0]
        else {
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
        let Statement::Expression(Expression::Call { arguments, .. }) = &program.statements[0]
        else {
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
        let Statement::Expression(Expression::Call { arguments, .. }) = &program.statements[0]
        else {
            panic!("expected a call expression");
        };
        assert!(matches!(
            &arguments[0],
            Expression::Convert { expression, target, .. }
                if target == "km" && matches!(expression.as_ref(), Expression::Variable { name, .. } if name == "distance")
        ));
    }
}
