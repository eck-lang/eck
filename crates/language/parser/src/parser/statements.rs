//! Statement recognition and variable declaration parsing.

use syntax::{
    Block, ConfigurationEntry, ConfigurationValue, RelationBinding, RelationCardinality,
    RelationDefinition, RelationRole, RelationRoleBinding, SourceIdentifier, Span, Statement,
    TypeDefinition, TypeField, UseClause, UseDeclaration, UseMember,
};

use crate::{ParseError, lexer::TokenKind};

use super::Parser;

impl Parser {
    /// Dispatches one declaration, control-flow statement, or expression.
    pub(super) fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if matches!(&self.peek().kind, TokenKind::Use) {
            return self.parse_use_declaration();
        }
        if matches!(&self.peek().kind, TokenKind::Type) {
            return self.parse_type_declaration();
        }
        if matches!(&self.peek().kind, TokenKind::Relation) {
            return self.parse_relation_statement();
        }
        if matches!(&self.peek().kind, TokenKind::Config) {
            return self.parse_configuration_statement();
        }
        if matches!(&self.peek().kind, TokenKind::If) {
            return self.parse_if_statement();
        }
        if self.starts_frame_declaration() {
            return self.parse_frame_declaration();
        }
        if self.starts_variable_declaration() {
            return self.parse_variable_declaration();
        }
        Ok(Statement::Expression(self.parse_expression(0)?))
    }

    /// Parses a namespace, selective-member, or wildcard `use` declaration.
    fn parse_use_declaration(&mut self) -> Result<Statement, ParseError> {
        let use_span = self.advance().span;
        let clause = match &self.peek().kind {
            TokenKind::LeftBrace => self.parse_member_use_clause()?,
            TokenKind::Star => self.parse_wildcard_use_clause()?,
            TokenKind::Ident(_) => {
                let namespace = self.parse_source_identifier("expected namespace after `use`")?;
                let alias = self.parse_optional_alias()?;
                UseClause::Namespace { namespace, alias }
            }
            _ => return Err(self.error_here("expected namespace, `{`, or `*` after `use`")),
        };
        let span = Span {
            start: use_span.start,
            end: self.previous().span.end,
        };
        Ok(Statement::Use(UseDeclaration {
            clause,
            use_span,
            span,
        }))
    }

    /// Parses `{ member [as alias], ... } from Namespace`.
    fn parse_member_use_clause(&mut self) -> Result<UseClause, ParseError> {
        self.advance();
        let mut members = Vec::new();
        if matches!(&self.peek().kind, TokenKind::RightBrace) {
            return Err(self.error_here("member imports cannot be empty"));
        }
        loop {
            let name = self.parse_source_identifier("expected imported member name")?;
            let alias = self.parse_optional_alias()?;
            let span = Span {
                start: name.span.start,
                end: alias.as_ref().map_or(name.span.end, |alias| alias.span.end),
            };
            members.push(UseMember { name, alias, span });
            if !matches!(&self.peek().kind, TokenKind::Comma) {
                break;
            }
            self.advance();
            if matches!(&self.peek().kind, TokenKind::RightBrace) {
                return Err(self.error_here("expected imported member after `,`"));
            }
        }
        self.expect_simple(TokenKind::RightBrace)?;
        self.expect_simple(TokenKind::From)?;
        let namespace = self.parse_source_identifier("expected namespace after `from`")?;
        Ok(UseClause::Members { namespace, members })
    }

    /// Parses `* [as Alias] from Namespace`.
    fn parse_wildcard_use_clause(&mut self) -> Result<UseClause, ParseError> {
        self.advance();
        let alias = self.parse_optional_alias()?;
        self.expect_simple(TokenKind::From)?;
        let namespace = self.parse_source_identifier("expected namespace after `from`")?;
        Ok(UseClause::Wildcard { namespace, alias })
    }

    /// Parses an optional `as Alias` suffix.
    fn parse_optional_alias(&mut self) -> Result<Option<SourceIdentifier>, ParseError> {
        if !matches!(&self.peek().kind, TokenKind::As) {
            return Ok(None);
        }
        self.advance();
        self.parse_source_identifier("expected alias after `as`")
            .map(Some)
    }

    /// Consumes one identifier while retaining its exact source span.
    fn parse_source_identifier(&mut self, message: &str) -> Result<SourceIdentifier, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Ident(name) => Ok(SourceIdentifier {
                name,
                span: token.span,
            }),
            _ => Err(self.error_at(token.span, message)),
        }
    }

    /// Parses a TypeScript-like structural row type declaration.
    fn parse_type_declaration(&mut self) -> Result<Statement, ParseError> {
        let start = self.advance().span.start;
        let name = self.expect_identifier("expected type name after `type`")?;
        self.skip_newlines();
        self.expect_simple(TokenKind::LeftBrace)?;
        self.skip_newlines();
        let mut fields = Vec::new();
        while !matches!(&self.peek().kind, TokenKind::RightBrace) {
            if matches!(&self.peek().kind, TokenKind::Eof) {
                return Err(self.error_here("expected `}` to close type declaration"));
            }
            let field_start = self.peek().span.start;
            let field_name = self.expect_identifier("expected field name")?;
            self.expect_simple(TokenKind::Colon)?;
            let type_name = self.expect_type_name("expected field type")?;
            fields.push(TypeField {
                name: field_name,
                type_name,
                span: Span {
                    start: field_start,
                    end: self.previous().span.end,
                },
            });
            self.finish_braced_line("type declaration")?;
        }
        let end = self.expect_simple(TokenKind::RightBrace)?.span.end;
        let span = Span { start, end };
        Ok(Statement::TypeDeclaration {
            definition: TypeDefinition { name, fields, span },
            span,
        })
    }

    /// Parses either a reusable relation definition or an explicit binding.
    fn parse_relation_statement(&mut self) -> Result<Statement, ParseError> {
        let start = self.advance().span.start;
        let name = self.expect_identifier("expected relation name after `relation`")?;
        if matches!(&self.peek().kind, TokenKind::Colon) {
            return self.parse_relation_binding(start, name);
        }
        self.parse_relation_definition(start, name)
    }

    /// Parses a reusable relation definition and its general boolean predicates.
    fn parse_relation_definition(
        &mut self,
        start: usize,
        name: String,
    ) -> Result<Statement, ParseError> {
        self.skip_newlines();
        self.expect_simple(TokenKind::LeftBrace)?;
        self.skip_newlines();
        let mut roles = Vec::new();
        while !matches!(&self.peek().kind, TokenKind::On) {
            if matches!(&self.peek().kind, TokenKind::RightBrace | TokenKind::Eof) {
                return Err(self.error_here("expected `on` block in relation definition"));
            }
            let role_start = self.peek().span.start;
            let role_name = self.expect_identifier("expected relation role name")?;
            self.expect_simple(TokenKind::Colon)?;
            let row_type_name = self.expect_type_name("expected relation role type")?;
            let cardinality = match self.advance().clone().kind {
                TokenKind::One => RelationCardinality::One,
                TokenKind::Many => RelationCardinality::Many,
                _ => {
                    return Err(self.error_at(
                        self.previous().span,
                        "invalid cardinality; expected `one` or `many`",
                    ));
                }
            };
            roles.push(RelationRole {
                name: role_name,
                row_type_name,
                cardinality,
                span: Span {
                    start: role_start,
                    end: self.previous().span.end,
                },
            });
            self.finish_braced_line("relation definition")?;
        }
        self.advance();
        self.skip_newlines();
        self.expect_simple(TokenKind::LeftBrace)?;
        self.skip_newlines();
        let mut predicates = Vec::new();
        while !matches!(&self.peek().kind, TokenKind::RightBrace) {
            if matches!(&self.peek().kind, TokenKind::Eof) {
                return Err(self.error_here("expected `}` to close relation predicate block"));
            }
            predicates.push(self.parse_expression(0)?);
            self.finish_braced_line("relation predicate block")?;
        }
        self.expect_simple(TokenKind::RightBrace)?;
        self.skip_newlines();
        let end = self.expect_simple(TokenKind::RightBrace)?.span.end;
        let span = Span { start, end };
        Ok(Statement::RelationDefinition {
            definition: RelationDefinition {
                name,
                roles,
                predicates,
                span,
            },
            span,
        })
    }

    /// Parses an explicit mapping from relation roles to concrete frames.
    fn parse_relation_binding(
        &mut self,
        start: usize,
        name: String,
    ) -> Result<Statement, ParseError> {
        self.advance();
        let definition_name =
            self.expect_identifier("expected relation definition name after `:`")?;
        self.skip_newlines();
        self.expect_simple(TokenKind::LeftBrace)?;
        self.skip_newlines();
        let mut roles = Vec::new();
        while !matches!(&self.peek().kind, TokenKind::RightBrace) {
            if matches!(&self.peek().kind, TokenKind::Eof) {
                return Err(self.error_here("expected `}` to close relation binding"));
            }
            let role_start = self.peek().span.start;
            let role_name = self.expect_identifier("expected relation role name")?;
            self.expect_simple(TokenKind::Equal)?;
            let frame_name = self.expect_identifier("expected frame name in relation binding")?;
            roles.push(RelationRoleBinding {
                role_name,
                frame_name,
                span: Span {
                    start: role_start,
                    end: self.previous().span.end,
                },
            });
            self.finish_braced_line("relation binding")?;
        }
        let end = self.expect_simple(TokenKind::RightBrace)?.span.end;
        let span = Span { start, end };
        Ok(Statement::RelationBinding {
            binding: RelationBinding {
                name,
                definition_name,
                roles,
                span,
            },
            span,
        })
    }

    /// Parses `name: frame RowType` with an optional future source expression.
    fn parse_frame_declaration(&mut self) -> Result<Statement, ParseError> {
        let start = self.peek().span.start;
        let name = self.expect_identifier("expected frame variable name")?;
        self.expect_simple(TokenKind::Colon)?;
        self.expect_simple(TokenKind::Frame)?;
        let row_type_name = self.expect_type_name("expected row type after `frame`")?;
        let expression = if matches!(&self.peek().kind, TokenKind::Equal) {
            self.advance();
            Some(self.parse_expression(0)?)
        } else {
            None
        };
        let end = expression
            .as_ref()
            .map(|expression| expression.span().end)
            .unwrap_or(self.previous().span.end);
        Ok(Statement::FrameDeclaration {
            name,
            row_type_name,
            expression,
            span: Span { start, end },
        })
    }

    /// Consumes a newline separator or accepts the closing brace at the cursor.
    fn finish_braced_line(&mut self, context: &str) -> Result<(), ParseError> {
        match &self.peek().kind {
            TokenKind::Newline => {
                self.skip_newlines();
                Ok(())
            }
            TokenKind::RightBrace => Ok(()),
            _ => Err(self.error_here(&format!("expected end of line or `}}` in {context}"))),
        }
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

    /// Reports whether the cursor begins `name: frame RowType`.
    fn starts_frame_declaration(&self) -> bool {
        matches!(&self.peek().kind, TokenKind::Ident(_))
            && matches!(
                self.peek_n(1).map(|token| &token.kind),
                Some(TokenKind::Colon)
            )
            && matches!(
                self.peek_n(2).map(|token| &token.kind),
                Some(TokenKind::Frame)
            )
    }

    /// Parses a typed variable declaration after its leading identifier was found.
    fn parse_variable_declaration(&mut self) -> Result<Statement, ParseError> {
        let start = self.peek().span.start;
        let name = self.expect_identifier("expected variable name")?;
        self.expect_simple(TokenKind::Colon)?;
        let type_name = self.expect_type_name("expected type name")?;
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
