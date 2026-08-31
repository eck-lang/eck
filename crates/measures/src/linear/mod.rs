use language_core::{CoreError, Extension, Registry};

use crate::common::{register_dimension, UnitDefinition};

/// Registers metric linear units expressed in meters.
pub struct LinearMeasureExtension;

const UNITS: &[UnitDefinition] = &[
    UnitDefinition {
        name: "kilometer",
        suffixes: &["km", "kilometer", "kilometers"],
        units_per_smallest: 1_000_000,
    },
    UnitDefinition {
        name: "hectometer",
        suffixes: &["hm", "hectometer", "hectometers"],
        units_per_smallest: 100_000,
    },
    UnitDefinition {
        name: "decameter",
        suffixes: &["dam", "decameter", "decameters"],
        units_per_smallest: 10_000,
    },
    UnitDefinition {
        name: "meter",
        suffixes: &["m", "meter", "meters"],
        units_per_smallest: 1_000,
    },
    UnitDefinition {
        name: "decimeter",
        suffixes: &["dm", "decimeter", "decimeters"],
        units_per_smallest: 100,
    },
    UnitDefinition {
        name: "centimeter",
        suffixes: &["cm", "centimeter", "centimeters"],
        units_per_smallest: 10,
    },
    UnitDefinition {
        name: "millimeter",
        suffixes: &["mm", "millimeter", "millimeters"],
        units_per_smallest: 1,
    },
];

impl Extension for LinearMeasureExtension {
    fn name(&self) -> &'static str {
        "linear-measure"
    }

    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        register_dimension(registry, UNITS)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
