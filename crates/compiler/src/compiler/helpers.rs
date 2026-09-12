//! Conversion helpers shared by the compiler modules.
//!
//! These free functions map source-level spellings onto registry contracts and
//! provide small span and literal predicates. They hold no compiler state.

use syntax::{BinaryOperator, Expression, Statement, UnaryOperator};

use ir::{TypedExpression, TypedExpressionKind};

/// Returns the source span for any root-only declaration or configuration statement.
pub(super) fn statement_span(statement: &Statement) -> syntax::Span {
    match statement {
        Statement::Use(declaration) => declaration.span,
        Statement::TypeDeclaration { span, .. }
        | Statement::FrameDeclaration { span, .. }
        | Statement::RelationDefinition { span, .. }
        | Statement::RelationBinding { span, .. }
        | Statement::Configuration { span, .. }
        | Statement::VariableDeclaration { span, .. }
        | Statement::BindingDeclaration { span, .. }
        | Statement::Assignment { span, .. }
        | Statement::If { span, .. }
        | Statement::While { span, .. }
        | Statement::For { span, .. }
        | Statement::Break { span }
        | Statement::Continue { span } => *span,
        Statement::Block(block) => block.span,
        Statement::Expression(expression) => expression.span(),
    }
}

/// Returns whether an expression is the literal boolean value `true`.
pub(super) fn is_true_boolean_literal(expression: &TypedExpression) -> bool {
    matches!(
        &expression.kind,
        TypedExpressionKind::Literal(value) if value.downcast_ref::<bool>() == Some(&true)
    )
}

/// Preserves the original expression while assigning the enclosing source span.
pub(super) fn with_span(mut expression: TypedExpression, span: syntax::Span) -> TypedExpression {
    expression.span = span;
    expression
}

/// Returns whether a power expression has a directly written negative integer exponent.
///
/// A negative integral exponent cannot produce an `int` in general. Compiling
/// both operands through the default fractional type gives `2 ** -1` a stable
/// decimal result while preserving the existing integer path for all other
/// integer powers.
pub(super) fn is_negative_integer_power(operator: &BinaryOperator, exponent: &Expression) -> bool {
    matches!(operator, BinaryOperator::Power)
        && matches!(
            exponent,
            Expression::Unary {
                operator: UnaryOperator::Negation,
                ..
            }
        )
}
