use syntax::Span;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{message} at {span:?}")]
pub struct ParseError {
    /// Human-readable explanation of the invalid source construct.
    pub message: String,
    /// Byte range in the source that caused the failure.
    pub span: Span,
}
