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
