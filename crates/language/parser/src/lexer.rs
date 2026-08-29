use logos::Logos;
use syntax::Span;

use crate::ParseError;

/// The lexical specification of ECK source code.
///
/// This enum deliberately contains only source-level rules. `lex` converts
/// its generated tokens into the parser's stable `Token` representation.
#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\r]+")]
enum RawTokenKind {
    #[regex(r"//[^\r\n]*", logos::skip, allow_greedy = true)]
    #[regex(r"/\*([^*]|\*+[^*/])*\*+/", logos::skip)]
    Comment,

    #[token("\n")]
    Newline,

    #[token(":")]
    Colon,
    #[token("=")]
    Equal,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("->")]
    Arrow,
    #[token("*")]
    Star,
    #[token("**")]
    DoubleStar,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("(")]
    LeftParenthesis,
    #[token(")")]
    RightParenthesis,
    #[token(",")]
    Comma,

    // Preserve the source spelling: each concrete type validates the numeric
    // literal only after the compiler resolves its expected type.
    #[regex(r"[0-9]+(?:\.[0-9]*)?(?:[eE][+-]?[0-9]*)?")]
    Number,

    #[token("true")]
    #[token("false")]
    Boolean,

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Ident,

    #[regex(r#""([^"\\]|\\[\s\S])*""#)]
    String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TokenKind {
    Ident(String),
    Number(String),
    String(String),
    Boolean(String),
    Colon,
    Equal,
    Plus,
    Minus,
    Arrow,
    Star,
    DoubleStar,
    Slash,
    Percent,
    LeftParenthesis,
    RightParenthesis,
    Comma,
    Newline,
    Eof,
}

#[derive(Clone, Debug)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: Span,
}

/// Converts source text into parser tokens while preserving byte spans.
///
/// The lexer keeps literal text intact except for supported string escapes, so
/// concrete types can validate numeric syntax after semantic resolution.
pub(crate) fn lex(source: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut lexer = RawTokenKind::lexer(source);

    while let Some(result) = lexer.next() {
        let range = lexer.span();
        let span = Span {
            start: range.start,
            end: range.end,
        };
        let raw_kind = result.map_err(|_| invalid_token_error(source, span))?;
        let kind = convert_raw_token(raw_kind, &source[range]);
        tokens.push(Token { kind, span });
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span {
            start: source.len(),
            end: source.len(),
        },
    });
    Ok(tokens)
}

/// Maps a Logos token to the parser representation, retaining its source text.
fn convert_raw_token(raw_kind: RawTokenKind, raw_text: &str) -> TokenKind {
    match raw_kind {
        RawTokenKind::Comment => unreachable!("comments are skipped by Logos"),
        RawTokenKind::Newline => TokenKind::Newline,
        RawTokenKind::Colon => TokenKind::Colon,
        RawTokenKind::Equal => TokenKind::Equal,
        RawTokenKind::Plus => TokenKind::Plus,
        RawTokenKind::Minus => TokenKind::Minus,
        RawTokenKind::Arrow => TokenKind::Arrow,
        RawTokenKind::Star => TokenKind::Star,
        RawTokenKind::DoubleStar => TokenKind::DoubleStar,
        RawTokenKind::Slash => TokenKind::Slash,
        RawTokenKind::Percent => TokenKind::Percent,
        RawTokenKind::LeftParenthesis => TokenKind::LeftParenthesis,
        RawTokenKind::RightParenthesis => TokenKind::RightParenthesis,
        RawTokenKind::Comma => TokenKind::Comma,
        RawTokenKind::Number => TokenKind::Number(raw_text.into()),
        RawTokenKind::Ident => TokenKind::Ident(raw_text.into()),
        RawTokenKind::String => TokenKind::String(decode_string(raw_text)),
        RawTokenKind::Boolean => TokenKind::Boolean(raw_text.into()),
    }
}

/// Decodes the escape sequences allowed inside an already matched string token.
fn decode_string(raw_text: &str) -> String {
    let content = &raw_text[1..raw_text.len() - 1];
    let mut decoded = String::new();
    let mut characters = content.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => decoded.push('\n'),
            Some('t') => decoded.push('\t'),
            Some('"') => decoded.push('"'),
            Some('\\') => decoded.push('\\'),
            Some(other) => decoded.push(other),
            None => unreachable!("a matched string cannot end with an escape"),
        }
    }
    decoded
}

/// Builds a precise lexical error, distinguishing unterminated strings.
fn invalid_token_error(source: &str, span: Span) -> ParseError {
    if source[span.start..].starts_with('"') {
        return ParseError {
            message: "unterminated string literal".into(),
            span: Span {
                start: span.start,
                end: source.len(),
            },
        };
    }

    ParseError {
        message: format!("unexpected character `{}`", &source[span.start..span.end]),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::{TokenKind, lex};
    use syntax::Span;

    #[test]
    fn lexes_eck_tokens_and_preserves_spans() {
        let tokens = lex("distance: decimal = 1.5m->km // convert\n").unwrap();

        assert_eq!(
            tokens.iter().map(|token| &token.kind).collect::<Vec<_>>(),
            vec![
                &TokenKind::Ident("distance".into()),
                &TokenKind::Colon,
                &TokenKind::Ident("decimal".into()),
                &TokenKind::Equal,
                &TokenKind::Number("1.5".into()),
                &TokenKind::Ident("m".into()),
                &TokenKind::Arrow,
                &TokenKind::Ident("km".into()),
                &TokenKind::Newline,
                &TokenKind::Eof,
            ]
        );
        assert_eq!(tokens[0].span, Span { start: 0, end: 8 });
        assert_eq!(tokens[4].span, Span { start: 20, end: 23 });
        assert_eq!(tokens[8].span, Span { start: 39, end: 40 });
    }

    #[test]
    fn skips_line_and_multiline_comments() {
        let tokens = lex("first: int = 1 // line\n/* block\ncomment */ second: int = 2\n").unwrap();

        assert_eq!(
            tokens.iter().map(|token| &token.kind).collect::<Vec<_>>(),
            vec![
                &TokenKind::Ident("first".into()),
                &TokenKind::Colon,
                &TokenKind::Ident("int".into()),
                &TokenKind::Equal,
                &TokenKind::Number("1".into()),
                &TokenKind::Newline,
                &TokenKind::Ident("second".into()),
                &TokenKind::Colon,
                &TokenKind::Ident("int".into()),
                &TokenKind::Equal,
                &TokenKind::Number("2".into()),
                &TokenKind::Newline,
                &TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn decodes_supported_string_escapes() {
        let tokens = lex(r#"print("first\n\"second\"")"#).unwrap();

        assert!(matches!(
            &tokens[2].kind,
            TokenKind::String(value) if value == "first\n\"second\""
        ));
    }

    #[test]
    fn reports_invalid_characters_and_unterminated_strings() {
        let invalid_character = lex("@").unwrap_err();
        assert_eq!(invalid_character.message, "unexpected character `@`");
        assert_eq!(invalid_character.span, Span { start: 0, end: 1 });

        let legacy_comment = lex("# legacy comment").unwrap_err();
        assert_eq!(legacy_comment.message, "unexpected character `#`");
        assert_eq!(legacy_comment.span, Span { start: 0, end: 1 });

        let unterminated_string = lex("\"missing").unwrap_err();
        assert_eq!(unterminated_string.message, "unterminated string literal");
        assert_eq!(unterminated_string.span, Span { start: 0, end: 8 });
    }
}
