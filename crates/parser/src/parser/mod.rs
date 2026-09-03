//! Token-stream parsing orchestration.
//!
//! Statement recognition, expression parsing, and cursor mechanics live in
//! sibling modules so each concern can evolve without growing one parser file.

use syntax::Program;

use crate::{
    ParseError,
    lexer::{Token, TokenKind},
};

mod expressions;
mod statements;
mod tokens;

/// Parses a complete, already-lexed ECK token stream.
pub(crate) struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Creates a parser positioned at the first token in a lexed source file.
    pub(crate) fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parses all statements until the lexer-provided end-of-file token.
    ///
    /// Statements must be separated by at least one newline unless the source
    /// ends immediately after the statement.
    pub(crate) fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        self.skip_newlines();
        while !matches!(&self.peek().kind, TokenKind::Eof) {
            statements.push(self.parse_statement()?);
            if !matches!(&self.peek().kind, TokenKind::Newline | TokenKind::Eof) {
                return Err(self.error_here("expected end of line"));
            }
            self.skip_newlines();
        }
        Ok(Program { statements })
    }
}
