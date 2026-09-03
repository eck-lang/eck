use language_core::{CoreError, Extension, Registry};

use crate::common::{UnitDefinition, register_dimension};

/// Registers metric volume units expressed in liters.
pub struct VolumeMeasureExtension;

const UNITS: &[UnitDefinition] = &[
    UnitDefinition {
        name: "kiloliter",
        suffixes: &["klt", "kiloliter", "kiloliters"],
        units_per_smallest: 1_000_000,
    },
    UnitDefinition {
        name: "hectoliter",
        suffixes: &["hlt", "hectoliter", "hectoliters"],
        units_per_smallest: 100_000,
    },
    UnitDefinition {
        name: "decaliter",
        suffixes: &["dalt", "decaliter", "decaliters"],
        units_per_smallest: 10_000,
    },
    UnitDefinition {
        name: "liter",
        suffixes: &["lt", "liter", "liters"],
        units_per_smallest: 1_000,
    },
    UnitDefinition {
        name: "deciliter",
        suffixes: &["dlt", "deciliter", "deciliters"],
        units_per_smallest: 100,
    },
    UnitDefinition {
        name: "centiliter",
        suffixes: &["clt", "centiliter", "centiliters"],
        units_per_smallest: 10,
    },
    UnitDefinition {
        name: "milliliter",
        suffixes: &["mlt", "milliliter", "milliliters"],
        units_per_smallest: 1,
    },
];

impl Extension for VolumeMeasureExtension {
    fn name(&self) -> &'static str {
        "volume-measure"
    }

    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        register_dimension(registry, UNITS)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
