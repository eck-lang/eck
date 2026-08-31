use language_core::{CoreError, Extension, Registry};

use crate::common::{register_dimension, UnitDefinition};

/// Registers metric mass units expressed in grams.
pub struct MassMeasureExtension;

const UNITS: &[UnitDefinition] = &[
    UnitDefinition {
        name: "kilogram",
        suffixes: &["kg", "kilogram", "kilograms"],
        units_per_smallest: 1_000_000,
    },
    UnitDefinition {
        name: "hectogram",
        suffixes: &["hg", "hectogram", "hectograms"],
        units_per_smallest: 100_000,
    },
    UnitDefinition {
        name: "decagram",
        suffixes: &["dag", "decagram", "decagrams"],
        units_per_smallest: 10_000,
    },
    UnitDefinition {
        name: "gram",
        suffixes: &["g", "gram", "grams"],
        units_per_smallest: 1_000,
    },
    UnitDefinition {
        name: "decigram",
        suffixes: &["dg", "decigram", "decigrams"],
        units_per_smallest: 100,
    },
    UnitDefinition {
        name: "centigram",
        suffixes: &["cg", "centigram", "centigrams"],
        units_per_smallest: 10,
    },
    UnitDefinition {
        name: "milligram",
        suffixes: &["mg", "milligram", "milligrams"],
        units_per_smallest: 1,
    },
];

impl Extension for MassMeasureExtension {
    fn name(&self) -> &'static str {
        "mass-measure"
    }

    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        register_dimension(registry, UNITS)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
