//! Pratt parsing for expressions, postfix conversions, and calls.

use syntax::{BinaryOperator, Expression, Span};

use crate::{ParseError, lexer::TokenKind};

use super::Parser;

impl Parser {
    /// Parses an expression whose next binary operator must meet `min_bp`.
    ///
    /// This Pratt parser assigns higher binding power to multiplication and
    /// division, and makes exponentiation right-associative.
    pub(super) fn parse_expression(&mut self, min_bp: u8) -> Result<Expression, ParseError> {
        let mut left_operand = self.parse_primary()?;
        loop {
            if matches!(&self.peek().kind, TokenKind::Arrow) {
                left_operand = self.parse_postfix_conversion(left_operand)?;
                continue;
            }

            let Some((operator, left_binding_power, right_binding_power)) =
                self.current_binary_operator()
            else {
                break;
            };
            if left_binding_power < min_bp {
                break;
            }
            self.advance();
            let right_operand = self.parse_expression(right_binding_power)?;
            let span = Span {
                start: left_operand.span().start,
                end: right_operand.span().end,
            };
            left_operand = Expression::Binary {
                operator,
                left_operand: Box::new(left_operand),
                right_operand: Box::new(right_operand),
                span,
            };
        }
        Ok(left_operand)
    }

    /// Parses a postfix `->unit` conversion when the cursor is at an arrow.
    fn parse_postfix_conversion(
        &mut self,
        expression: Expression,
    ) -> Result<Expression, ParseError> {
        self.advance();
        let target_start = self.peek().span.start;
        let target = self
            .expect_identifier("expected conversion target after `->`")
            .map_err(|mut error| {
                error.span.start = target_start;
                error
            })?;
        let span = Span {
            start: expression.span().start,
            end: self.previous().span.end,
        };
        Ok(Expression::Convert {
            expression: Box::new(expression),
            target,
            span,
        })
    }

    /// Returns the binary operator and Pratt binding powers at the cursor.
    fn current_binary_operator(&self) -> Option<(BinaryOperator, u8, u8)> {
        match &self.peek().kind {
            TokenKind::Plus => Some((BinaryOperator::Addition, 1, 2)),
            TokenKind::Minus => Some((BinaryOperator::Subtraction, 1, 2)),
            TokenKind::Star => Some((BinaryOperator::Multiplication, 3, 4)),
            TokenKind::Slash => Some((BinaryOperator::Division, 3, 4)),
            TokenKind::Percent => Some((BinaryOperator::Remainder, 3, 4)),
            TokenKind::DoubleStar => Some((BinaryOperator::Power, 5, 5)),
            _ => None,
        }
    }

    /// Parses one literal, variable, call, or parenthesized expression.
    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Number(raw_text) => Ok(self.parse_number_literal(token.span, raw_text)),
            TokenKind::String(value) => Ok(Expression::String {
                value,
                span: token.span,
            }),
            TokenKind::Ident(name) => self.parse_identifier_expression(token.span, name),
            TokenKind::LeftParenthesis => {
                let expression = self.parse_expression(0)?;
                self.expect_simple(TokenKind::RightParenthesis)?;
                Ok(expression)
            }
            _ => Err(self.error_at(token.span, "expected expression")),
        }
    }

    /// Parses a numeric literal and its optional adjacent unit suffix.
    fn parse_number_literal(&mut self, span: Span, raw_text: String) -> Expression {
        let mut span = span;
        let suffix = match &self.peek().kind {
            TokenKind::Ident(value) => {
                let value = value.clone();
                span.end = self.advance().span.end;
                Some(value)
            }
            _ => None,
        };
        Expression::Number {
            raw_text,
            suffix,
            span,
        }
    }

    /// Parses an identifier as either a call or a variable reference.
    fn parse_identifier_expression(
        &mut self,
        span: Span,
        name: String,
    ) -> Result<Expression, ParseError> {
        if matches!(&self.peek().kind, TokenKind::LeftParenthesis) {
            return self.parse_call(span.start, name);
        }
        Ok(Expression::Variable { name, span })
    }

    /// Parses a comma-separated argument list following a call name.
    fn parse_call(&mut self, start: usize, name: String) -> Result<Expression, ParseError> {
        self.expect_simple(TokenKind::LeftParenthesis)?;
        let mut arguments = Vec::new();
        if !matches!(&self.peek().kind, TokenKind::RightParenthesis) {
            loop {
                arguments.push(self.parse_expression(0)?);
                if !matches!(&self.peek().kind, TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }
        let end = self.expect_simple(TokenKind::RightParenthesis)?.span.end;
        Ok(Expression::Call {
            name,
            arguments,
            span: Span { start, end },
        })
    }
}
