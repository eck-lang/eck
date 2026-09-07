pub mod data;
pub mod frequency;
pub mod linear;
pub mod mass;
pub mod percentage;
pub mod time;
pub mod volume;

mod common;

pub use data::DataMeasureExtension;
pub use frequency::FrequencyMeasureExtension;
pub use linear::LinearMeasureExtension;
pub use mass::MassMeasureExtension;
pub use percentage::PercentageMeasureExtension;
pub use time::TimeMeasureExtension;
pub use volume::VolumeMeasureExtension;

use language_core::{CoreError, Extension, Registry};

/// Aggregates all metric measure dimensions.
pub struct MeasuresExtension;

impl Extension for MeasuresExtension {
    fn name(&self) -> &'static str {
        "measures"
    }

    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        LinearMeasureExtension.register(registry)?;
        VolumeMeasureExtension.register(registry)?;
        MassMeasureExtension.register(registry)?;
        FrequencyMeasureExtension.register(registry)?;
        TimeMeasureExtension.register(registry)?;
        DataMeasureExtension.register(registry)?;
        // Percentage composes with every previously registered subtype, so
        // it runs after all measure dimensions.
        PercentageMeasureExtension.register(registry)?;
        Ok(())
    }
}
