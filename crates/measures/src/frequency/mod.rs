use language_core::{CoreError, Extension, Registry};

use crate::common::{UnitDefinition, register_dimension};

/// Registers frequency units expressed in hertz.
pub struct FrequencyMeasureExtension;

const UNITS: &[UnitDefinition] = &[
    UnitDefinition {
        name: "terahertz",
        suffixes: &["thz", "terahertz", "terahertzs"],
        units_per_smallest: 1_000_000_000_000,
    },
    UnitDefinition {
        name: "gigahertz",
        suffixes: &["ghz", "gigahertz", "gigahertzs"],
        units_per_smallest: 1_000_000_000,
    },
    UnitDefinition {
        name: "megahertz",
        suffixes: &["mhz", "megahertz", "megahertzs"],
        units_per_smallest: 1_000_000,
    },
    UnitDefinition {
        name: "kilohertz",
        suffixes: &["khz", "kilohertz", "kilohertzs"],
        units_per_smallest: 1_000,
    },
    UnitDefinition {
        name: "hertz",
        suffixes: &["hz", "hertz", "hertzs"],
        units_per_smallest: 1,
    },
];

impl Extension for FrequencyMeasureExtension {
    fn name(&self) -> &'static str {
        "frequency-measure"
    }

    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        register_dimension(registry, UNITS)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
