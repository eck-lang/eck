//! Cursor movement, token expectations, and parser error construction.

use syntax::Span;

use crate::{
    ParseError,
    lexer::{Token, TokenKind},
};

use super::Parser;

impl Parser {
    /// Advances past every consecutive newline at the cursor.
    pub(super) fn skip_newlines(&mut self) {
        while matches!(&self.peek().kind, TokenKind::Newline) {
            self.advance();
        }
    }

    /// Consumes a punctuation token with the same variant as `expected`.
    pub(super) fn expect_simple(&mut self, expected: TokenKind) -> Result<Token, ParseError> {
        let token = self.advance().clone();
        if std::mem::discriminant(&token.kind) == std::mem::discriminant(&expected) {
            return Ok(token);
        }
        Err(self.error_at(token.span, format!("expected {expected:?}")))
    }

    /// Consumes an identifier and returns its source spelling.
    pub(super) fn expect_identifier(&mut self, message: &str) -> Result<String, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Ident(value) => Ok(value),
            _ => Err(self.error_at(token.span, message)),
        }
    }

    /// Consumes a type name which may be a keyword such as `null`.
    pub(super) fn expect_type_name(&mut self, message: &str) -> Result<String, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Ident(value) => Ok(value),
            TokenKind::Null => Ok("null".into()),
            _ => Err(self.error_at(token.span, message)),
        }
    }

    /// Returns the token at the current cursor position.
    pub(super) fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    /// Returns a token relative to the cursor without moving it.
    pub(super) fn peek_n(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    /// Returns the current token and moves to the next token when possible.
    pub(super) fn advance(&mut self) -> &Token {
        let index = self.pos;
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        &self.tokens[index]
    }

    /// Returns the token consumed immediately before the current cursor.
    pub(super) fn previous(&self) -> &Token {
        &self.tokens[self.pos.saturating_sub(1)]
    }

    /// Builds an error anchored at the token currently under the cursor.
    pub(super) fn error_here(&self, message: &str) -> ParseError {
        self.error_at(self.peek().span, message)
    }

    /// Builds an error with an explicit source span.
    pub(super) fn error_at(&self, span: Span, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            span,
        }
    }
}
