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

    #[token("@config")]
    Config,

    #[token(":")]
    Colon,
    #[token("==")]
    EqualEqual,
    #[token("!=")]
    BangEqual,
    #[token("<=")]
    LessEqual,
    #[token(">=")]
    GreaterEqual,
    #[token("=")]
    Equal,
    #[token("<")]
    Less,
    #[token(">")]
    Greater,
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
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token(",")]
    Comma,

    #[token("if")]
    If,

    // Preserve the source spelling: each concrete type validates the numeric
    // literal only after the compiler resolves its expected type.
    #[regex(r"[0-9]+(?:\.[0-9]*)?(?:[eE][+-]?[0-9]*)?")]
    Number,

    #[token("true")]
    #[token("false")]
    Boolean,

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Ident,

    #[regex(r#""([^"\\\r\n]|\\[\s\S])*""#)]
    DoubleQuotedString,
    #[regex(r"'([^'\\\r\n]|\\[\s\S])*'")]
    SingleQuotedString,
    #[regex(r"`([^`\\]|\\[\s\S])*`")]
    BacktickString,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TokenKind {
    Ident(String),
    Number(String),
    String(String),
    Boolean(String),
    Colon,
    Equal,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Plus,
    Minus,
    Arrow,
    Star,
    DoubleStar,
    Slash,
    Percent,
    LeftParenthesis,
    RightParenthesis,
    LeftBrace,
    RightBrace,
    Comma,
    If,
    Newline,
    Config,
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
        let kind = convert_raw_token(raw_kind, &source[range], span)?;
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
fn convert_raw_token(
    raw_kind: RawTokenKind,
    raw_text: &str,
    span: Span,
) -> Result<TokenKind, ParseError> {
    Ok(match raw_kind {
        RawTokenKind::Comment => unreachable!("comments are skipped by Logos"),
        RawTokenKind::Newline => TokenKind::Newline,
        RawTokenKind::Config => TokenKind::Config,
        RawTokenKind::Colon => TokenKind::Colon,
        RawTokenKind::Equal => TokenKind::Equal,
        RawTokenKind::EqualEqual => TokenKind::EqualEqual,
        RawTokenKind::BangEqual => TokenKind::BangEqual,
        RawTokenKind::Less => TokenKind::Less,
        RawTokenKind::LessEqual => TokenKind::LessEqual,
        RawTokenKind::Greater => TokenKind::Greater,
        RawTokenKind::GreaterEqual => TokenKind::GreaterEqual,
        RawTokenKind::Plus => TokenKind::Plus,
        RawTokenKind::Minus => TokenKind::Minus,
        RawTokenKind::Arrow => TokenKind::Arrow,
        RawTokenKind::Star => TokenKind::Star,
        RawTokenKind::DoubleStar => TokenKind::DoubleStar,
        RawTokenKind::Slash => TokenKind::Slash,
        RawTokenKind::Percent => TokenKind::Percent,
        RawTokenKind::LeftParenthesis => TokenKind::LeftParenthesis,
        RawTokenKind::RightParenthesis => TokenKind::RightParenthesis,
        RawTokenKind::LeftBrace => TokenKind::LeftBrace,
        RawTokenKind::RightBrace => TokenKind::RightBrace,
        RawTokenKind::Comma => TokenKind::Comma,
        RawTokenKind::If => TokenKind::If,
        RawTokenKind::Number => TokenKind::Number(raw_text.into()),
        RawTokenKind::Ident => TokenKind::Ident(raw_text.into()),
        RawTokenKind::DoubleQuotedString => TokenKind::String(decode_string(raw_text, '"', span)?),
        RawTokenKind::SingleQuotedString => TokenKind::String(decode_string(raw_text, '\'', span)?),
        RawTokenKind::BacktickString => TokenKind::String(decode_string(raw_text, '`', span)?),
        RawTokenKind::Boolean => TokenKind::Boolean(raw_text.into()),
    })
}

/// Decodes the escape sequences allowed inside an already matched string token.
fn decode_string(raw_text: &str, delimiter: char, span: Span) -> Result<String, ParseError> {
    let content = &raw_text[1..raw_text.len() - 1];
    let mut decoded = String::new();
    let mut characters = content.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => decoded.push('\n'),
            Some('r') => decoded.push('\r'),
            Some('t') => decoded.push('\t'),
            Some('0') => decoded.push('\0'),
            Some('\\') => decoded.push('\\'),
            Some('u') => decoded.push(decode_unicode_escape(&mut characters, span)?),
            Some(escaped_delimiter) if escaped_delimiter == delimiter => {
                decoded.push(escaped_delimiter);
            }
            Some(other) => {
                return Err(string_literal_error(
                    span,
                    format!("unknown string escape `\\{other}`"),
                ));
            }
            None => unreachable!("a matched string cannot end with an escape"),
        }
    }
    Ok(decoded)
}

/// Decodes one `\\u{...}` escape after its leading `u` has been consumed.
fn decode_unicode_escape(
    characters: &mut std::iter::Peekable<std::str::Chars<'_>>,
    span: Span,
) -> Result<char, ParseError> {
    if characters.next() != Some('{') {
        return Err(string_literal_error(
            span,
            "Unicode escapes must use `\\u{...}`",
        ));
    }

    let mut digits = String::new();
    loop {
        match characters.next() {
            Some('}') if digits.is_empty() => {
                return Err(string_literal_error(
                    span,
                    "Unicode escapes require at least one hexadecimal digit",
                ));
            }
            Some('}') => break,
            Some(character) if character.is_ascii_hexdigit() && digits.len() < 6 => {
                digits.push(character);
            }
            Some(_) => {
                return Err(string_literal_error(
                    span,
                    "Unicode escapes accept at most six hexadecimal digits",
                ));
            }
            None => {
                return Err(string_literal_error(
                    span,
                    "unterminated Unicode escape in string literal",
                ));
            }
        }
    }

    let scalar =
        u32::from_str_radix(&digits, 16).expect("validated hexadecimal Unicode escape must parse");
    char::from_u32(scalar).ok_or_else(|| {
        string_literal_error(span, "Unicode escape is not a valid Unicode scalar value")
    })
}

/// Builds a lexical error covering one fully matched but invalid string token.
fn string_literal_error(span: Span, message: impl Into<String>) -> ParseError {
    ParseError {
        message: message.into(),
        span,
    }
}

/// Builds a precise lexical error, distinguishing unterminated strings.
fn invalid_token_error(source: &str, span: Span) -> ParseError {
    if source[span.start..].starts_with(['"', '\'', '`']) {
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
#[path = "lexer.tests.rs"]
mod tests;
