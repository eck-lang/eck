use super::*;
use crate::lexer::lex;
use syntax::{ConfigurationValue, Expression, Program, Statement};

fn parse(source: &str) -> Result<Program, ParseError> {
    Parser::new(lex(source)?).parse_program()
}

#[test]
fn parses_multiline_and_nested_if_statements() {
    let program = parse(
        "if (true) {\n\
         value: int = 1\n\
         if (value == 1) { print(value) }\n\
         }\n",
    )
    .unwrap();

    let Statement::If {
        condition, body, ..
    } = &program.statements[0]
    else {
        panic!("expected an if statement");
    };
    assert!(matches!(condition, Expression::Boolean { raw_text, .. } if raw_text == "true"));
    assert_eq!(body.statements.len(), 2);
    assert!(matches!(body.statements[1], Statement::If { .. }));
}

#[test]
fn accepts_a_newline_between_the_condition_and_block() {
    let program = parse("if (true)\n{\nprint(1)\n}\n").unwrap();

    assert!(matches!(program.statements[0], Statement::If { .. }));
}

#[test]
fn accepts_an_empty_block_and_preserves_if_and_block_spans() {
    let program = parse("if (true) {}").unwrap();

    let Statement::If { body, span, .. } = &program.statements[0] else {
        panic!("expected an if statement");
    };
    assert!(body.statements.is_empty());
    assert_eq!(*span, syntax::Span { start: 0, end: 12 });
    assert_eq!(body.span, syntax::Span { start: 10, end: 12 });
}

#[test]
fn rejects_unclosed_and_non_separated_blocks() {
    let unclosed = parse("if (true) {\nprint(1)\n").unwrap_err();
    assert!(unclosed.message.contains("expected `}`"));

    let non_separated = parse("if (true) { print(1) print(2) }\n").unwrap_err();
    assert!(
        non_separated
            .message
            .contains("expected end of line or `}`")
    );
}

#[test]
fn rejects_missing_condition_parentheses_and_opening_brace() {
    for (source, expected) in [
        ("if true) {}", "expected LeftParenthesis"),
        ("if (true {}", "expected RightParenthesis"),
        ("if (true)\nprint(1)", "expected LeftBrace"),
    ] {
        let error = parse(source).unwrap_err();
        assert!(
            error.message.contains(expected),
            "unexpected error for `{source}`: {error}"
        );
    }
}

/// Verifies that a root configuration directive preserves its nested object structure.
#[test]
fn parses_nested_root_configuration_directives() {
    let program = parse(
        "@config {\n\
         decimal: {\n\
         precision: 29\n\
         rounding: HalfEven\n\
         format: { scale: 4\nrounding: Truncate }\n\
         }\n\
         }\n",
    )
    .unwrap();

    let Statement::Configuration { entries, .. } = &program.statements[0] else {
        panic!("expected a configuration directive");
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "decimal");
    let ConfigurationValue::Object {
        entries: decimal_entries,
        ..
    } = &entries[0].value
    else {
        panic!("expected the decimal configuration object");
    };
    assert!(matches!(
        decimal_entries[0].value,
        ConfigurationValue::Number { ref raw_text, .. } if raw_text == "29"
    ));
    assert!(matches!(
        decimal_entries[1].value,
        ConfigurationValue::Symbol { ref name, .. } if name == "HalfEven"
    ));
    assert!(matches!(
        decimal_entries[2].value,
        ConfigurationValue::Object { .. }
    ));
}

/// Verifies that configuration directives cannot be nested in control-flow blocks.
#[test]
fn rejects_configuration_directives_inside_blocks() {
    let error = parse("if (true) {\n@config { decimal: { precision: 4 } }\n}\n").unwrap_err();

    assert!(error.message.contains("only allowed at the root level"));
}
