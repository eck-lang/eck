//! The built-in array container.
//!
//! This module is the single owner of the array payload and its storage, the
//! element contract a value crosses into storage through, the end operations,
//! element access, and the formatter installed by [`ArrayExtension`]. The array
//! vocabulary itself ([`ArrayType`] and [`ArrayElementContract`]).

pub(crate) mod compiler;
mod contract;
mod end_operations;
mod formatting;
mod runtime;
mod storage;
mod types;
mod value;

pub use contract::{apply_element_contract, element_crosses_unchanged};
pub use end_operations::{apply_end_operation, element_at, set_element};
pub use types::{ArrayElementContract, ArrayEndOperation, ArrayType, StaticArrayElementContract};
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
#[path = "mod.tests.rs"]
mod tests;
