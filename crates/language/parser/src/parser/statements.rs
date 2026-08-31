//! Statement recognition and variable declaration parsing.

use syntax::{Block, Span, Statement};

use crate::{ParseError, lexer::TokenKind};

use super::Parser;

impl Parser {
    /// Parses either a typed variable declaration or a standalone expression.
    pub(super) fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if matches!(&self.peek().kind, TokenKind::If) {
            return self.parse_if_statement();
        }
        if self.starts_variable_declaration() {
            return self.parse_variable_declaration();
        }
        Ok(Statement::Expression(self.parse_expression(0)?))
    }

    /// Parses an `if (condition) { statements }` control-flow statement.
    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        let start = self.advance().span.start;
        self.expect_simple(TokenKind::LeftParenthesis)?;
        let condition = self.parse_expression(0)?;
        self.expect_simple(TokenKind::RightParenthesis)?;
        self.skip_newlines();
        let body = self.parse_block()?;
        let span = Span {
            start,
            end: body.span.end,
        };
        Ok(Statement::If {
            condition,
            body,
            span,
        })
    }

    /// Parses a brace-delimited sequence of newline-separated statements.
    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let opening_brace = self.expect_simple(TokenKind::LeftBrace)?;
        let mut statements = Vec::new();
        self.skip_newlines();

        while !matches!(&self.peek().kind, TokenKind::RightBrace) {
            if matches!(&self.peek().kind, TokenKind::Eof) {
                return Err(self.error_here("expected `}` to close block"));
            }

            statements.push(self.parse_statement()?);
            match &self.peek().kind {
                TokenKind::Newline => self.skip_newlines(),
                TokenKind::RightBrace => {}
                _ => return Err(self.error_here("expected end of line or `}`")),
            }
        }

        let closing_brace = self.expect_simple(TokenKind::RightBrace)?;
        Ok(Block {
            statements,
            span: Span {
                start: opening_brace.span.start,
                end: closing_brace.span.end,
            },
        })
    }

    /// Reports whether the cursor begins the `name: type = expression` form.
    fn starts_variable_declaration(&self) -> bool {
        matches!(&self.peek().kind, TokenKind::Ident(_))
            && matches!(
                self.peek_n(1).map(|token| &token.kind),
                Some(TokenKind::Colon)
            )
    }

    /// Parses a typed variable declaration after its leading identifier was found.
    fn parse_variable_declaration(&mut self) -> Result<Statement, ParseError> {
        let start = self.peek().span.start;
        let name = self.expect_identifier("expected variable name")?;
        self.expect_simple(TokenKind::Colon)?;
        let type_name = self.expect_identifier("expected type name")?;
        self.expect_simple(TokenKind::Equal)?;
        let expression = self.parse_expression(0)?;
        let end = expression.span().end;
        Ok(Statement::VariableDecl {
            name,
            type_name,
            expression,
            span: Span { start, end },
        })
    }
}

#[cfg(test)]
#[path = "statements.tests.rs"]
mod tests;
