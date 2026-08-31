pub mod frequency;
pub mod linear;
pub mod mass;
pub mod volume;

mod common;

pub use frequency::FrequencyMeasureExtension;
pub use linear::LinearMeasureExtension;
pub use mass::MassMeasureExtension;
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
        Ok(())
    }
}
