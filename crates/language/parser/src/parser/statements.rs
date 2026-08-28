//! Statement recognition and variable declaration parsing.

use syntax::{Span, Statement};

use crate::{ParseError, lexer::TokenKind};

use super::Parser;

impl Parser {
    /// Parses either a typed variable declaration or a standalone expression.
    pub(super) fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if self.starts_variable_declaration() {
            return self.parse_variable_declaration();
        }
        Ok(Statement::Expression(self.parse_expression(0)?))
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
