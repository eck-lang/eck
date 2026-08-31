use language_core::{CoreError, Extension, Registry};

use crate::common::{register_dimension, UnitDefinition};

/// Registers data size units expressed in bytes.
pub struct DataMeasureExtension;

const UNITS: &[UnitDefinition] = &[
    UnitDefinition {
        name: "petabyte",
        suffixes: &["PB", "petabyte", "petabytes"],
        units_per_smallest: 1_000_000_000_000_000,
    },
    UnitDefinition {
        name: "terabyte",
        suffixes: &["TB", "terabyte", "terabytes"],
        units_per_smallest: 1_000_000_000_000,
    },
    UnitDefinition {
        name: "gigabyte",
        suffixes: &["GB", "gigabyte", "gigabytes"],
        units_per_smallest: 1_000_000_000,
    },
    UnitDefinition {
        name: "megabyte",
        suffixes: &["MB", "megabyte", "megabytes"],
        units_per_smallest: 1_000_000,
    },
    UnitDefinition {
        name: "kilobyte",
        suffixes: &["KB", "kilobyte", "kilobytes"],
        units_per_smallest: 1_000,
    },
    UnitDefinition {
        name: "byte",
        suffixes: &["B", "byte", "bytes"],
        units_per_smallest: 1,
    },
];

impl Extension for DataMeasureExtension {
    fn name(&self) -> &'static str {
        "data-measure"
    }

    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        register_dimension(registry, UNITS)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
