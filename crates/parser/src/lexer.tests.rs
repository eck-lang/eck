use super::{TokenKind, lex};
use syntax::Span;

#[test]
fn lexes_eck_tokens_and_preserves_spans() {
    let tokens = lex("distance: decimal = 1.5m->km // convert\n").unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::Ident(name) if name == "distance"));
    assert_eq!(tokens[0].span, Span { start: 0, end: 8 });
    assert_eq!(tokens[4].span, Span { start: 20, end: 23 });
    assert_eq!(tokens[8].span, Span { start: 39, end: 40 });
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
