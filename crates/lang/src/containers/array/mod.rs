mod contract;
mod end_operations;
mod formatting;
mod storage;
mod value;

pub use crate::semantic::{ArrayElementMode, ArrayType};
pub use contract::apply_element_contract;
pub use end_operations::{apply_end_operation, element_at, set_element};
pub use value::ArrayValue;

use crate::semantic::{ArrayValueFormatter, CoreError, Extension, Registry};

/// Registers the array payload formatter without adding a scalar type.
pub struct ArrayExtension;

impl Extension for ArrayExtension {
    /// Returns the stable extension name used by dialect diagnostics.
    fn name(&self) -> &'static str {
        "array"
    }

    /// Installs the one formatter required to render array values.
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let formatter: ArrayValueFormatter = formatting::format_value;
        registry.register_array_formatter(formatter)
    }
}

#[cfg(test)]
#[path = "lib.tests.rs"]
mod tests;
