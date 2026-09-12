use super::{TokenKind, lex};
use syntax::Span;

#[test]
fn lexes_eck_tokens_and_preserves_spans() {
    let tokens = lex("distance: decimal = 1.5m->to(km) // convert\n").unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::Ident(name) if name == "distance"));
    assert_eq!(tokens[0].span, Span { start: 0, end: 8 });
    assert!(matches!(&tokens[4].kind, TokenKind::Number(raw) if raw == "1.5"));
    assert!(
        tokens
            .iter()
            .any(|token| matches!(&token.kind, TokenKind::Arrow))
    );
    assert!(
        tokens
            .iter()
            .any(|token| matches!(&token.kind, TokenKind::Ident(name) if name == "to"))
    );
}

#[test]
fn lexes_all_comparison_operators_with_their_full_spans() {
    let tokens = lex("a == b != c < d <= e > f >= g").unwrap();
    let kinds = tokens.iter().map(|token| &token.kind).collect::<Vec<_>>();
    assert!(matches!(
        kinds.as_slice(),
        [
            TokenKind::Ident(_),
            TokenKind::EqualEqual,
            TokenKind::Ident(_),
            TokenKind::BangEqual,
            TokenKind::Ident(_),
            TokenKind::Less,
            TokenKind::Ident(_),
            TokenKind::LessEqual,
            TokenKind::Ident(_),
            TokenKind::Greater,
            TokenKind::Ident(_),
            TokenKind::GreaterEqual,
            TokenKind::Ident(_),
            TokenKind::Eof
        ]
    ));
    assert_eq!(tokens[1].span, Span { start: 2, end: 4 });
    assert_eq!(tokens[11].span, Span { start: 25, end: 27 });
}

#[test]
fn decodes_strings_and_reports_invalid_tokens() {
    assert!(
        matches!(&lex(r#"print("first\n")"#).unwrap()[2].kind, TokenKind::String(value) if value == "first\n")
    );
    assert_eq!(lex("@").unwrap_err().message, "unexpected character `@`");
    assert_eq!(
        lex("\"missing").unwrap_err().message,
        "unterminated string literal"
    );
}

/// Verifies delimiter-specific escapes for single, double, and backtick strings.
#[test]
fn decodes_every_string_delimiter_and_its_escape_sequences() {
    let tokens = lex(r#"print('L\'acqua')
print("disse: \"ciao\"")
print(`quote: ' and ", backtick: \``)
print("\r\t\0\\\u{1F600}")"#)
    .unwrap();
    let values = tokens
        .iter()
        .filter_map(|token| match &token.kind {
            TokenKind::String(value) => Some(value.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        values,
        [
            "L'acqua",
            "disse: \"ciao\"",
            "quote: ' and \", backtick: `",
            "\r\t\0\\😀",
        ]
    );
}

/// Verifies that backtick strings retain physical line breaks and indentation.
#[test]
fn preserves_multiline_backtick_content() {
    let tokens = lex("value: string = `first\n  second\nthird`\n").unwrap();

    assert!(
        matches!(&tokens[4].kind, TokenKind::String(value) if value == "first\n  second\nthird")
    );
}

/// Verifies that ordinary quoted strings remain single-line literals.
#[test]
fn rejects_physical_newlines_in_single_and_double_quoted_strings() {
    for source in ["'first\nsecond'", "\"first\nsecond\""] {
        assert_eq!(
            lex(source).unwrap_err().message,
            "unterminated string literal"
        );
    }
}

/// Verifies strict validation of unknown and malformed escape sequences.
#[test]
fn rejects_invalid_string_escape_sequences() {
    for source in [r#""\q""#, r#"'\"'"#, r#""\u{}""#, r#""\u{110000}""#] {
        assert!(
            lex(source).is_err(),
            "source unexpectedly succeeded: {source}"
        );
    }
}

#[test]
fn skips_line_and_multiline_comments() {
    let tokens = lex("first: int = 1 // line\n/* block\ncomment */ second: int = 2\n").unwrap();

    assert!(matches!(
        tokens
            .iter()
            .map(|token| &token.kind)
            .collect::<Vec<_>>()
            .as_slice(),
        [
            TokenKind::Ident(_),
            TokenKind::Colon,
            TokenKind::Ident(_),
            TokenKind::Equal,
            TokenKind::Number(_),
            TokenKind::Newline,
            TokenKind::Ident(_),
            TokenKind::Colon,
            TokenKind::Ident(_),
            TokenKind::Equal,
            TokenKind::Number(_),
            TokenKind::Newline,
            TokenKind::Eof,
        ]
    ));
}

#[test]
fn lexes_if_and_braces_without_claiming_prefixed_identifiers() {
    let tokens = lex("if (iffy) { print(iffy) }").unwrap();
    let kinds = tokens.iter().map(|token| &token.kind).collect::<Vec<_>>();

    assert!(matches!(
        kinds.as_slice(),
        [
            TokenKind::If,
            TokenKind::LeftParenthesis,
            TokenKind::Ident(name),
            TokenKind::RightParenthesis,
            TokenKind::LeftBrace,
            TokenKind::Ident(_),
            TokenKind::LeftParenthesis,
            TokenKind::Ident(_),
            TokenKind::RightParenthesis,
            TokenKind::RightBrace,
            TokenKind::Eof,
        ] if name == "iffy"
    ));
}

/// Verifies conditional and loop keywords do not claim longer identifiers.
#[test]
fn lexes_conditional_and_loop_control_keywords() {
    let tokens = lex("else while break continue elsewhere meanwhile breaker continued !").unwrap();
    let kinds = tokens.iter().map(|token| &token.kind).collect::<Vec<_>>();

    assert!(matches!(
        kinds.as_slice(),
        [
            TokenKind::Else,
            TokenKind::While,
            TokenKind::Break,
            TokenKind::Continue,
            TokenKind::Ident(elsewhere),
            TokenKind::Ident(meanwhile_name),
            TokenKind::Ident(breaker),
            TokenKind::Ident(continued),
            TokenKind::Bang,
            TokenKind::Eof,
        ] if elsewhere == "elsewhere"
            && meanwhile_name == "meanwhile"
            && breaker == "breaker"
            && continued == "continued"
    ));
}

/// Verifies binding keywords do not claim identifiers with longer names.
#[test]
fn lexes_binding_keywords() {
    let tokens = lex("let const letter constant").unwrap();
    let kinds = tokens.iter().map(|token| &token.kind).collect::<Vec<_>>();

    assert!(matches!(
        kinds.as_slice(),
        [
            TokenKind::Let,
            TokenKind::Const,
            TokenKind::Ident(letter),
            TokenKind::Ident(constant),
            TokenKind::Eof,
        ] if letter == "letter" && constant == "constant"
    ));
}

/// Verifies `for (i in 0..10) {}` lexes `..` as one token with exact spans.
///
/// The range operator must not split into two dots, the bounds must not
/// swallow a dot into the number, and `for`/`in` must remain usable as
/// identifier prefixes elsewhere.
#[test]
fn lexes_for_range_with_single_dot_dot_token() {
    let tokens = lex("for (i in 0..10) {}").unwrap();
    let kinds = tokens.iter().map(|token| &token.kind).collect::<Vec<_>>();

    assert!(matches!(
        kinds.as_slice(),
        [
            TokenKind::For,
            TokenKind::LeftParenthesis,
            TokenKind::Ident(variable),
            TokenKind::In,
            TokenKind::Number(start),
            TokenKind::DotDot,
            TokenKind::Number(end),
            TokenKind::RightParenthesis,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::Eof,
        ] if variable == "i" && start == "0" && end == "10"
    ));
    assert_eq!(tokens[0].span, Span { start: 0, end: 3 });
    assert_eq!(tokens[4].span, Span { start: 10, end: 11 });
    assert_eq!(tokens[5].span, Span { start: 11, end: 13 });
    assert_eq!(tokens[6].span, Span { start: 13, end: 15 });
}

/// Verifies range token repair preserves trailing-decimal numeric literals.
#[test]
fn distinguishes_range_separators_from_trailing_decimal_points() {
    let tokens = lex("value: decimal = 1.\nfor (i in 0..1) {}").unwrap();

    assert!(matches!(&tokens[4].kind, TokenKind::Number(number) if number == "1."));
    assert!(matches!(&tokens[10].kind, TokenKind::Number(number) if number == "0"));
    assert!(matches!(tokens[11].kind, TokenKind::DotDot));
    assert!(matches!(&tokens[12].kind, TokenKind::Number(number) if number == "1"));
}

/// Verifies `for` and `in` keywords do not claim longer identifiers.
#[test]
fn lexes_for_in_prefixes_as_identifiers() {
    let tokens = lex("format inside").unwrap();
    let kinds = tokens.iter().map(|token| &token.kind).collect::<Vec<_>>();

    assert!(matches!(
        kinds.as_slice(),
        [TokenKind::Ident(first), TokenKind::Ident(second), TokenKind::Eof,]
            if first == "format" && second == "inside"
    ));
}
