//! Compilation of lazy source contracts and resolved row field slots.

use std::sync::Arc;

use crate::CompileError;
use crate::ir::{TypedExpression, TypedExpressionKind};
use crate::semantic::{SemanticType, SourceType, ValueType};
use crate::syntax::{Expression, Span, TypeExpression};

use super::Compiler;

impl Compiler<'_> {
    /// Applies `->as(Row[])` as a lazy row contract, never as an array boundary.
    pub(super) fn compile_source_as(
        &mut self,
        expression: &Expression,
        target_type: &TypeExpression,
        span: Span,
    ) -> Result<TypedExpression, CompileError> {
        let typed_source = self.compile_expression(expression, None)?;
        if !matches!(typed_source.output, Some(SemanticType::Source(_))) {
            return Err(CompileError::new(
                expression.span(),
                "`->as` requires a lazy source",
            ));
        }
        let TypeExpression::Array { element, .. } = target_type else {
            return Err(CompileError::new(
                target_type.span(),
                "a source row contract must be a row array type such as `User[]`",
            ));
        };
        let declared = self.resolve_declared_type(element, element.span())?;
        let SemanticType::Row(row_type) = declared.semantic_type else {
            return Err(CompileError::new(
                element.span(),
                "a source row contract must name a structural row type",
            ));
        };
        Ok(TypedExpression {
            kind: TypedExpressionKind::SourceAs {
                source: Box::new(typed_source),
                row_type: row_type.clone(),
            },
            output: Some(SemanticType::Source(Arc::new(SourceType {
                row: Some(row_type),
            }))),
            span,
        })
    }

    /// Resolves `.field` to an integer row slot at compile time.
    pub(super) fn compile_row_field(
        &mut self,
        expression: &Expression,
        field: &str,
        span: Span,
    ) -> Result<TypedExpression, CompileError> {
        let typed_row = self.compile_expression(expression, None)?;
        if matches!(typed_row.output, Some(SemanticType::Open)) {
            let string_type = self
                .registry
                .default_string()
                .map_err(|error| CompileError::core(span, error))?;
            return Ok(TypedExpression {
                kind: TypedExpressionKind::DynamicRowField {
                    row: Box::new(typed_row),
                    field: field.to_string(),
                },
                output: Some(SemanticType::Scalar(ValueType::plain(string_type))),
                span,
            });
        }
        let Some(SemanticType::Row(row_type)) = typed_row.output.as_ref() else {
            return Err(CompileError::new(
                expression.span(),
                "field access requires a row",
            ));
        };
        let field_index = row_type
            .field_index(field)
            .ok_or_else(|| CompileError::new(span, format!("unknown row field `{field}`")))?;
        let output = row_type.fields[field_index].semantic_type.clone();
        Ok(TypedExpression {
            kind: TypedExpressionKind::RowField {
                row: Box::new(typed_row),
                field_index,
            },
            output: Some(output),
            span,
        })
    }
}
