use crate::semantic::CoreError;
use crate::syntax::Span;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{message} at {span:?}")]
pub struct CompileError {
    pub message: String,
    pub span: Span,
}

impl CompileError {
    pub(crate) fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }

    pub(crate) fn core(span: Span, error: CoreError) -> Self {
        Self::new(span, error.to_string())
    }
}
