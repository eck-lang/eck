use language_core::{CoreError, Extension, Registry};

use crate::common::{UnitDefinition, register_dimension};

/// Registers time units expressed in durations.
pub struct TimeMeasureExtension;

const UNITS: &[UnitDefinition] = &[
    UnitDefinition {
        name: "day",
        suffixes: &["d", "day", "days"],
        units_per_smallest: 86_400_000_000_000,
    },
    UnitDefinition {
        name: "hour",
        suffixes: &["h", "hour", "hours"],
        units_per_smallest: 3_600_000_000_000,
    },
    UnitDefinition {
        name: "minute",
        suffixes: &["min", "minute", "minutes"],
        units_per_smallest: 60_000_000_000,
    },
    UnitDefinition {
        name: "second",
        suffixes: &["s", "second", "seconds"],
        units_per_smallest: 1_000_000_000,
    },
    UnitDefinition {
        name: "millisecond",
        suffixes: &["ms", "millisecond", "milliseconds"],
        units_per_smallest: 1_000_000,
    },
    UnitDefinition {
        name: "microsecond",
        suffixes: &["us", "microsecond", "microseconds"],
        units_per_smallest: 1_000,
    },
    UnitDefinition {
        name: "nanosecond",
        suffixes: &["ns", "nanosecond", "nanoseconds"],
        units_per_smallest: 1,
    },
];

impl Extension for TimeMeasureExtension {
    fn name(&self) -> &'static str {
        "time-measure"
    }

    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        register_dimension(registry, UNITS)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
