//! Evaluation of lazy source contracts and resolved row fields.

use std::sync::Arc;

use crate::RuntimeError;
use crate::connectors::source::CsvSource;
use crate::ir::TypedExpression;
use crate::semantic::{RowType, Value};

use super::Runtime;

impl Runtime<'_> {
    /// Retypes an unopened source without consuming a CSV record.
    pub(super) fn eval_source_as(
        &mut self,
        expression: &TypedExpression,
        row_type: Arc<RowType>,
    ) -> Result<Option<Value>, RuntimeError> {
        let value = self
            .eval(expression)?
            .ok_or_else(|| RuntimeError::Message("source contract has no input value".into()))?;
        let source = value
            .downcast_ref::<CsvSource>()
            .ok_or_else(|| RuntimeError::Message("source contract has invalid input".into()))?;
        Ok(Some(source.clone().with_row_type(row_type).into_value()))
    }

    /// Reads one compiler-resolved field from an owned semantic row.
    pub(super) fn eval_row_field(
        &mut self,
        expression: &TypedExpression,
        field_index: usize,
    ) -> Result<Option<Value>, RuntimeError> {
        let value = self
            .eval(expression)?
            .ok_or_else(|| RuntimeError::Message("row field has no input value".into()))?;
        let fields = value
            .downcast_ref::<Vec<Value>>()
            .ok_or_else(|| RuntimeError::Message("row field has invalid input".into()))?;
        fields
            .get(field_index)
            .cloned()
            .map(Some)
            .ok_or_else(|| RuntimeError::Message("row field index is missing".into()))
    }

    /// Resolves a header field on an untyped row and returns its textual cell.
    pub(super) fn eval_dynamic_row_field(
        &mut self,
        expression: &TypedExpression,
        field: &str,
    ) -> Result<Option<Value>, RuntimeError> {
        let value = self
            .eval(expression)?
            .ok_or_else(|| RuntimeError::Message("row field has no input value".into()))?;
        let row_type = value
            .row_type()
            .ok_or_else(|| RuntimeError::Message("field access requires a row".into()))?;
        let index = row_type
            .field_index(field)
            .ok_or_else(|| RuntimeError::Message(format!("unknown CSV field `{field}`")))?;
        let fields = value
            .downcast_ref::<Vec<Value>>()
            .ok_or_else(|| RuntimeError::Message("row field has invalid input".into()))?;
        fields
            .get(index)
            .cloned()
            .map(Some)
            .ok_or_else(|| RuntimeError::Message("row field index is missing".into()))
    }
}
