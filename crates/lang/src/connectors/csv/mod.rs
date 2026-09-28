//! Streaming CSV records for a later typed decoding stage.

// The batch cursor is an internal consumption seam for future tabular readers.
#[allow(dead_code)]
mod reader;
mod runtime;

use crate::semantic::{CoreError, Extension, Registry};

/// Registers the built-in uppercase CSV namespace.
pub struct CsvExtension;

impl Extension for CsvExtension {
    /// Names the connector extension.
    fn name(&self) -> &'static str {
        "CSV"
    }

    /// Installs CSV.read through the shared registry.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        runtime::register(registry)
    }
}

#[allow(unused_imports)]
pub(crate) use reader::{CsvConfiguration, CsvReader, CsvRecordBatch};
