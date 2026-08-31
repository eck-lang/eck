//! Pratt parsing for expressions, postfix conversions, and calls.

use syntax::{
    BinaryOperator, ComparisonOperator, Expression, FrameLiteralColumn, LogicalOperator, Span,
    UnaryOperator,
};

use crate::{ParseError, lexer::TokenKind};

use super::Parser;

impl Parser {
    /// Parses an expression while preserving the legacy Pratt entry point.
    ///
    /// A zero binding power parses the complete logical-expression grammar.
    /// Recursive arithmetic parsing uses nonzero binding powers internally.
    pub(super) fn parse_expression(&mut self, min_bp: u8) -> Result<Expression, ParseError> {
        if min_bp == 0 {
            return self.parse_logical_or();
        }
        self.parse_arithmetic(min_bp)
    }

    /// Parses `||` expressions, which have the lowest expression precedence.
    fn parse_logical_or(&mut self) -> Result<Expression, ParseError> {
        let mut left_operand = self.parse_logical_and()?;
        while matches!(&self.peek().kind, TokenKind::PipePipe) {
            self.advance();
            let right_operand = self.parse_logical_and()?;
            let span = Span {
                start: left_operand.span().start,
                end: right_operand.span().end,
            };
            left_operand = Expression::Logical {
                operator: LogicalOperator::Or,
                left_operand: Box::new(left_operand),
                right_operand: Box::new(right_operand),
                span,
            };
        }
        Ok(left_operand)
    }

    /// Parses `&&` expressions above logical OR and below comparisons.
    fn parse_logical_and(&mut self) -> Result<Expression, ParseError> {
        let mut left_operand = self.parse_comparison()?;
        while matches!(&self.peek().kind, TokenKind::AmpersandAmpersand) {
            self.advance();
            let right_operand = self.parse_comparison()?;
            let span = Span {
                start: left_operand.span().start,
                end: right_operand.span().end,
            };
            left_operand = Expression::Logical {
                operator: LogicalOperator::And,
                left_operand: Box::new(left_operand),
                right_operand: Box::new(right_operand),
                span,
            };
        }
        Ok(left_operand)
    }

    /// Parses one optional non-associative comparison.
    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut left_operand = self.parse_arithmetic(0)?;
        if let Some(operator) = self.current_comparison_operator() {
            self.advance();
            let right_operand = self.parse_arithmetic(0)?;
            let span = Span {
                start: left_operand.span().start,
                end: right_operand.span().end,
            };
            left_operand = Expression::Comparison {
                operator,
                left_operand: Box::new(left_operand),
                right_operand: Box::new(right_operand),
                span,
            };
            if self.current_comparison_operator().is_some() {
                return Err(self.error_here(
                    "chained comparisons are not supported; parenthesize each comparison",
                ));
            }
        }
        Ok(left_operand)
    }

    /// Parses arithmetic with Pratt binding powers and postfix operations.
    fn parse_arithmetic(&mut self, min_bp: u8) -> Result<Expression, ParseError> {
        let mut left_operand = self.parse_primary()?;
        loop {
            if matches!(&self.peek().kind, TokenKind::Dot) {
                left_operand = self.parse_postfix_field_access(left_operand)?;
                continue;
            }
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
            let right_operand = self.parse_arithmetic(right_binding_power)?;
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

    /// Parses a postfix `.field` access without materializing a row object.
    fn parse_postfix_field_access(
        &mut self,
        expression: Expression,
    ) -> Result<Expression, ParseError> {
        self.advance();
        let field = self.expect_identifier("expected field name after `.`")?;
        let span = Span {
            start: expression.span().start,
            end: self.previous().span.end,
        };
        Ok(Expression::FieldAccess {
            expression: Box::new(expression),
            field,
            span,
        })
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
            TokenKind::Minus => self.parse_negation(token.span),
            TokenKind::String(value) => Ok(Expression::String {
                value,
                span: token.span,
            }),
            TokenKind::Boolean(raw_text) => Ok(Expression::Boolean {
                raw_text,
                span: token.span,
            }),
            TokenKind::Null => Ok(Expression::Null { span: token.span }),
            TokenKind::Ident(name) => self.parse_identifier_expression(token.span, name),
            TokenKind::Frame => self.parse_frame_literal(token.span.start),
            TokenKind::LeftParenthesis => {
                let expression = self.parse_expression(0)?;
                self.expect_simple(TokenKind::RightParenthesis)?;
                Ok(expression)
            }
            _ => Err(self.error_at(token.span, "expected expression")),
        }
    }

    /// Parses `frame { column: [values] }` into explicit column-oriented syntax.
    fn parse_frame_literal(&mut self, start: usize) -> Result<Expression, ParseError> {
        self.skip_newlines();
        self.expect_simple(TokenKind::LeftBrace)?;
        self.skip_newlines();
        let mut columns = Vec::new();
        while !matches!(&self.peek().kind, TokenKind::RightBrace) {
            if matches!(&self.peek().kind, TokenKind::Eof) {
                return Err(self.error_here("expected `}` to close frame literal"));
            }
            let column_start = self.peek().span.start;
            let name = self.expect_identifier("expected frame column name")?;
            self.expect_simple(TokenKind::Colon)?;
            self.expect_simple(TokenKind::LeftBracket)?;
            let mut values = Vec::new();
            if !matches!(&self.peek().kind, TokenKind::RightBracket) {
                loop {
                    values.push(self.parse_expression(0)?);
                    if !matches!(&self.peek().kind, TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                }
            }
            let column_end = self.expect_simple(TokenKind::RightBracket)?.span.end;
            columns.push(FrameLiteralColumn {
                name,
                values,
                span: Span {
                    start: column_start,
                    end: column_end,
                },
            });
            match &self.peek().kind {
                TokenKind::Newline => self.skip_newlines(),
                TokenKind::RightBrace => {}
                _ => return Err(self.error_here("expected end of line or `}` in frame literal")),
            }
        }
        let end = self.expect_simple(TokenKind::RightBrace)?.span.end;
        Ok(Expression::FrameLiteral {
            columns,
            span: Span { start, end },
        })
    }

    /// Parses a numeric literal and its optional suffix.
    ///
    /// Identifier suffixes retain their existing whitespace-tolerant syntax,
    /// while `%` is a suffix only when it is immediately adjacent to the
    /// literal. This keeps spaced percent signs available as the remainder
    /// operator.
    fn parse_number_literal(&mut self, span: Span, raw_text: String) -> Expression {
        let mut span = span;
        let suffix = match &self.peek().kind {
            TokenKind::Ident(value) => {
                let value = value.clone();
                span.end = self.advance().span.end;
                Some(value)
            }
            TokenKind::Percent if self.peek().span.start == span.end => {
                span.end = self.advance().span.end;
                Some("%".into())
            }
            _ => None,
        };
        Expression::Number {
            raw_text,
            suffix,
            span,
        }
    }

    /// Parses negation with lower precedence than a power and higher precedence than products.
    ///
    /// This gives `-2 ** 2` the conventional interpretation `-(2 ** 2)`,
    /// while still allowing a negative exponent such as `2 ** -1`.
    fn parse_negation(&mut self, start: Span) -> Result<Expression, ParseError> {
        if matches!(self.peek().kind, TokenKind::Minus) {
            return Err(self.error_at(
                self.peek().span,
                "`--` is not supported; write `-(-expression)` for double negation",
            ));
        }
        let operand = self.parse_arithmetic(5)?;
        let span = Span {
            start: start.start,
            end: operand.span().end,
        };
        Ok(Expression::Unary {
            operator: UnaryOperator::Negation,
            operand: Box::new(operand),
            span,
        })
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

    /// Returns the non-associative comparison operator at the cursor.
    fn current_comparison_operator(&self) -> Option<ComparisonOperator> {
        match &self.peek().kind {
            TokenKind::EqualEqual => Some(ComparisonOperator::Equal),
            TokenKind::BangEqual => Some(ComparisonOperator::NotEqual),
            TokenKind::Less => Some(ComparisonOperator::Less),
            TokenKind::LessEqual => Some(ComparisonOperator::LessOrEqual),
            TokenKind::Greater => Some(ComparisonOperator::Greater),
            TokenKind::GreaterEqual => Some(ComparisonOperator::GreaterOrEqual),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "expressions.tests.rs"]
mod tests;
