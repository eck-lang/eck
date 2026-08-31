use language_core::{CoreError, Extension, Registry};

use crate::common::{register_dimension, UnitDefinition};

/// Registers data size units expressed in bits.
pub struct DataMeasureExtension;

const UNITS: &[UnitDefinition] = &[
    UnitDefinition {
        name: "pebibyte",
        suffixes: &["PiB", "pebibyte", "pebibytes"],
        units_per_smallest: 9_007_199_254_740_992,
    },
    UnitDefinition {
        name: "petabyte",
        suffixes: &["PB", "petabyte", "petabytes"],
        units_per_smallest: 8_000_000_000_000_000,
    },
    UnitDefinition {
        name: "tebibyte",
        suffixes: &["TiB", "tebibyte", "tebibytes"],
        units_per_smallest: 8_796_093_022_208,
    },
    UnitDefinition {
        name: "terabyte",
        suffixes: &["TB", "terabyte", "terabytes"],
        units_per_smallest: 8_000_000_000_000,
    },
    UnitDefinition {
        name: "gibibyte",
        suffixes: &["GiB", "gibibyte", "gibibytes"],
        units_per_smallest: 8_589_934_592,
    },
    UnitDefinition {
        name: "gigabyte",
        suffixes: &["GB", "gigabyte", "gigabytes"],
        units_per_smallest: 8_000_000_000,
    },
    UnitDefinition {
        name: "mebibyte",
        suffixes: &["MiB", "mebibyte", "mebibytes"],
        units_per_smallest: 8_388_608,
    },
    UnitDefinition {
        name: "megabyte",
        suffixes: &["MB", "megabyte", "megabytes"],
        units_per_smallest: 8_000_000,
    },
    UnitDefinition {
        name: "kibibyte",
        suffixes: &["KiB", "kibibyte", "kibibytes"],
        units_per_smallest: 8_192,
    },
    UnitDefinition {
        name: "kilobyte",
        suffixes: &["KB", "kilobyte", "kilobytes"],
        units_per_smallest: 8_000,
    },
    UnitDefinition {
        name: "byte",
        suffixes: &["B", "byte", "bytes"],
        units_per_smallest: 8,
    },
    UnitDefinition {
        name: "bit",
        suffixes: &["b", "bit", "bits"],
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
