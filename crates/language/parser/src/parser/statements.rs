//! Statement recognition and variable declaration parsing.

use syntax::{Block, ConfigurationEntry, ConfigurationValue, Span, Statement};

use crate::{ParseError, lexer::TokenKind};

use super::Parser;

impl Parser {
    /// Parses either a typed variable declaration or a standalone expression.
    pub(super) fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if matches!(&self.peek().kind, TokenKind::Config) {
            return self.parse_configuration_statement();
        }
        if matches!(&self.peek().kind, TokenKind::If) {
            return self.parse_if_statement();
        }
        if self.starts_variable_declaration() {
            return self.parse_variable_declaration();
        }
        Ok(Statement::Expression(self.parse_expression(0)?))
    }

    /// Parses a root-level `@config { ... }` modal configuration override.
    fn parse_configuration_statement(&mut self) -> Result<Statement, ParseError> {
        let start = self.advance().span.start;
        self.skip_newlines();
        let ConfigurationValue::Object { entries, span } = self.parse_configuration_object()?
        else {
            unreachable!("configuration object parser always returns an object")
        };
        Ok(Statement::Configuration {
            entries,
            span: Span {
                start,
                end: span.end,
            },
        })
    }

    /// Parses a brace-delimited configuration object and its nested entries.
    fn parse_configuration_object(&mut self) -> Result<ConfigurationValue, ParseError> {
        let opening_brace = self.expect_simple(TokenKind::LeftBrace)?;
        let mut entries = Vec::new();
        self.skip_newlines();

        while !matches!(&self.peek().kind, TokenKind::RightBrace) {
            if matches!(&self.peek().kind, TokenKind::Eof) {
                return Err(self.error_here("expected `}` to close configuration object"));
            }
            entries.push(self.parse_configuration_entry()?);
            match &self.peek().kind {
                TokenKind::Newline => self.skip_newlines(),
                TokenKind::RightBrace => {}
                _ => return Err(self.error_here("expected end of line or `}`")),
            }
        }

        let closing_brace = self.expect_simple(TokenKind::RightBrace)?;
        Ok(ConfigurationValue::Object {
            entries,
            span: Span {
                start: opening_brace.span.start,
                end: closing_brace.span.end,
            },
        })
    }

    /// Parses one `name: value` entry from a configuration object.
    fn parse_configuration_entry(&mut self) -> Result<ConfigurationEntry, ParseError> {
        let start = self.peek().span.start;
        let name = self.expect_identifier("expected configuration name")?;
        self.expect_simple(TokenKind::Colon)?;
        let value = self.parse_configuration_value()?;
        Ok(ConfigurationEntry {
            name,
            span: Span {
                start,
                end: value.span().end,
            },
            value,
        })
    }

    /// Parses a numeric, enum-like, or nested object configuration value.
    fn parse_configuration_value(&mut self) -> Result<ConfigurationValue, ParseError> {
        match self.peek().kind.clone() {
            TokenKind::Number(raw_text) => {
                let span = self.advance().span;
                Ok(ConfigurationValue::Number { raw_text, span })
            }
            TokenKind::Minus => {
                let minus_span = self.advance().span;
                let number_token = self.advance().clone();
                let TokenKind::Number(raw_text) = number_token.kind else {
                    return Err(self.error_at(
                        number_token.span,
                        "expected a number after `-` in configuration value",
                    ));
                };
                Ok(ConfigurationValue::Number {
                    raw_text: format!("-{raw_text}"),
                    span: Span {
                        start: minus_span.start,
                        end: number_token.span.end,
                    },
                })
            }
            TokenKind::Ident(name) => {
                let span = self.advance().span;
                Ok(ConfigurationValue::Symbol { name, span })
            }
            TokenKind::LeftBrace => self.parse_configuration_object(),
            _ => Err(self.error_here("expected configuration value")),
        }
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

            if matches!(&self.peek().kind, TokenKind::Config) {
                return Err(self.error_here("`@config` is only allowed at the root level"));
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
